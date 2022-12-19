use env_var::env_var;
use once_cell::sync::Lazy;
use oracle::Connection;
use service::external_scaler_server::ExternalScaler;
use service::external_scaler_server::ExternalScalerServer;
use service::GetMetricSpecResponse;
use service::GetMetricsRequest;
use service::GetMetricsResponse;
use service::IsActiveResponse;
use service::MetricSpec;
use service::MetricValue;
use service::ScaledObjectRef;
use std::panic;
use std::process;
use std::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod service {
    tonic::include_proto!("externalscaler");
}

static DB_CONNECTION: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let url = env_var!(required "DB_URL");
    let username = env_var!(required "DB_USER");
    let password = env_var!(required "DB_PASSWORD");
    let conn = Connection::connect(username, password, url).unwrap();
    Mutex::new(conn)
});

async fn run_query(query: &str) -> i64 {
    match DB_CONNECTION.lock() {
        Ok(conn) => match conn.query(query, &[]) {
            Ok(rows) => {
                let mut metric_value: i64 = 0;
                let mut counter = 0;
                for row_result in rows {
                    let row = row_result.unwrap();
                    counter += 1;
                    if counter == 1 {
                        let first_column: Result<i64, _> = row.get(0);
                        match first_column {
                            Ok(value) => metric_value = value,
                            Err(_) => metric_value = counter,
                        }
                    } else {
                        metric_value = counter;
                    }
                }
                return metric_value;
            }
            Err(err) => match err {
                oracle::Error::DpiError(err) => {
                    if err.message() == "DPI-1010: not connected" {
                        panic!("connection lost");
                    } else {
                        log::error!("Error on statement: {} => {:?}", query, err);
                        return 0;
                    }
                }
                _ => {
                    log::error!("Error on statement: {} => {:?}", query, err);
                    return 0;
                }
            },
        },
        Err(_e) => {
            log::error!("poison error, restarting...");
            std::process::exit(1);
        }
    }
}

#[derive(Debug)]
struct ScalerService;

#[tonic::async_trait]
impl ExternalScaler for ScalerService {
    async fn get_metrics(
        &self,
        request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let inner = request.into_inner();
        let name = inner.metric_name;
        let scaled_object = inner.scaled_object_ref.unwrap();
        let metadata = scaled_object.scaler_metadata.clone();
        log::debug!("get_metrics: {name}: {:?}", metadata);
        if metadata.contains_key("query") {
            let query = metadata.get("query").unwrap();
            let metric_value = run_query(query).await;
            log::debug!("get_metrics: {metric_value} from query \"{query}\"");
            return Ok(Response::new(GetMetricsResponse {
                metric_values: vec![MetricValue {
                    metric_name: name,
                    metric_value: metric_value,
                }],
            }));
        } else {
            log::warn!(
                "get_metrics: no query found in metadata: {:?}",
                scaled_object
            );
            Ok(Response::new(GetMetricsResponse {
                metric_values: vec![MetricValue {
                    metric_name: name,
                    metric_value: 0,
                }],
            }))
        }
    }
    async fn is_active(
        &self,
        request: Request<ScaledObjectRef>,
    ) -> Result<Response<IsActiveResponse>, Status> {
        let inner = request.into_inner();
        let name = inner.name;
        let metadata = inner.scaler_metadata.clone();
        let mut metric_value: i64 = 0;
        log::debug!("is_active: {name}: {:?}", metadata);
        if metadata.contains_key("query") {
            let query = metadata.get("query").unwrap();
            metric_value = run_query(query).await;
            log::debug!("is_active: {metric_value} from query \"{query}\"");
        } else {
            log::warn!("is_active: no query found in metadata: {:?}", metadata);
        }
        if metric_value > 0 {
            Ok(Response::new(IsActiveResponse { result: true }))
        } else {
            Ok(Response::new(IsActiveResponse { result: false }))
        }
    }
    async fn stream_is_active(
        &self,
        _request: Request<ScaledObjectRef>,
    ) -> Result<Response<Self::StreamIsActiveStream>, Status> {
        log::debug!("stream_is_active: not implemented");
        Err(Status::unimplemented("stream_is_active is not implemented"))
    }
    async fn get_metric_spec(
        &self,
        request: Request<ScaledObjectRef>,
    ) -> Result<Response<GetMetricSpecResponse>, Status> {
        let inner = request.into_inner();
        let name = inner.name.clone();
        log::debug!("get_metric_spec: {:?}", inner);
        let resp = MetricSpec {
            metric_name: name,
            target_size: 1,
        };
        log::debug!("returning: {:?}", resp);
        let result = GetMetricSpecResponse {
            metric_specs: vec![resp],
        };
        Ok(Response::new(result))
    }
    type StreamIsActiveStream = ReceiverStream<Result<IsActiveResponse, Status>>;
}

#[tokio::main]
async fn main() {
    env_logger::init();
    {
        let _ = DB_CONNECTION.lock();
    }

    //add panic hook to shutdown engine on error
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        log::error!("receiving panic hook, shutting down engine");
        orig_hook(panic_info);
        process::exit(1);
    }));

    log::info!("listening on port 10000");
    let addr = "0.0.0.0:10000".parse().unwrap();

    let scaler_svc = ScalerService {};

    let svc = ExternalScalerServer::new(scaler_svc);

    Server::builder()
        .add_service(svc)
        .serve(addr)
        .await
        .expect("starting server failed");
}
