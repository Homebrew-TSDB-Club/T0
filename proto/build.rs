fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .format(true)
        .out_dir("src")
        .compile(&["idl/prometheus/service.proto"], &["idl/prometheus"])?;
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .format(true)
        .out_dir("src")
        .compile(&["idl/ping/service.proto"], &["idl/ping"])?;
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .format(true)
        .out_dir("src")
        .compile(&["idl/query/service.proto"], &["idl/query"])?;
    Ok(())
}
