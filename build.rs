fn main() {
    // tonic_build::configure()
    //      .build_server(true)
    //      .build_client(false)
    //      .compile(
    //          &["proto/externalscaler.proto"],
    //          &["proto"],
    //      ).unwrap();
    tonic_build::compile_protos("proto/externalscaler.proto").unwrap();
}
