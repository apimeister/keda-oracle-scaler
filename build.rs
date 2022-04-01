fn main() {
    tonic_build::configure()
         .build_server(true)
         .build_client(false)
         .compile(
             &["proto/externalscaler.proto"],
             &["proto"],
         ).unwrap();
 }