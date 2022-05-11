FROM rust as builder
RUN apt-get update && apt install cmake -y
RUN rustup component add rustfmt clippy
WORKDIR /usr/lib/oracle
RUN wget https://download.oracle.com/otn_software/linux/instantclient/instantclient-basiclite-linuxx64.zip
RUN unzip -j instantclient-basiclite-linuxx64.zip
WORKDIR /app
ADD . .
RUN cargo build --release

FROM debian:stable-slim as runner
ENV RUST_LOG=info
ENV LD_LIBRARY_PATH=/usr/lib/oracle
RUN apt-get update && apt-get install -y libaio1 && apt-get clean
COPY --from=builder /usr/lib/oracle /usr/lib/oracle
COPY --from=builder /app/target/release/keda-oracle-scaler /keda-oracle-scaler
CMD ["/keda-oracle-scaler"]