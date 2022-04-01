
use service::external_scaler_server::ExternalScaler;
use service::GetMetricSpecResponse;
use service::GetMetricsRequest;
use service::GetMetricsResponse;
use service::IsActiveResponse;
use service::MetricSpec;
use service::MetricValue;
use service::ScaledObjectRef;
use service::external_scaler_server::ExternalScalerServer;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

pub mod service {
    tonic::include_proto!("externalscaler");
}

#[derive(Debug)]
struct ScalerService;

#[tonic::async_trait]
impl ExternalScaler for ScalerService {
    async fn get_metrics(
        &self,
        request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let params = request.metadata();
        println!("{:?}", params);
        Ok(Response::new(GetMetricsResponse {
            metric_values: vec![MetricValue{
                metric_name: "count".to_string(),
                metric_value: 0,
            }],
        }))
    }
    async fn is_active(
        &self,
        request: Request<ScaledObjectRef>,
    ) -> Result<Response<IsActiveResponse>, Status> {
        let params = request.metadata();
        println!("{:?}", params);
        Ok(Response::new(IsActiveResponse {
            result: true,
        }))
    }
    async fn stream_is_active(
        &self,
        request: Request<ScaledObjectRef>,
    ) -> Result<Response<Self::StreamIsActiveStream>, Status> {
        let params = request.metadata();
        println!("{:?}", params);
        Err(Status::unimplemented("stream_is_active is not implemented"))
    }
    async fn get_metric_spec(
        &self,
        _request: Request<ScaledObjectRef>,
    ) -> Result<Response<GetMetricSpecResponse>, Status> {
        let result = GetMetricSpecResponse {
            metric_specs: vec![MetricSpec{
                metric_name: "count".to_string(),
                target_size: 1,
            }],
        };
        Ok(Response::new(result))
    }
    type StreamIsActiveStream = ReceiverStream<Result<IsActiveResponse, Status>>;
}

#[tokio::main]
async fn main() {
    env_logger::init();
    log::info!("listening on port 10000");
    let addr = "[::1]:10000".parse().unwrap();

    let scaler_svc = ScalerService{};

    let svc = ExternalScalerServer::new(scaler_svc);

    Server::builder().add_service(svc).serve(addr).await.expect("starting server failed");
}
