use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var("SKIP_PROTO_BUILD").is_ok() {
        return Ok(());
    }
    tonic_build::configure()
        .out_dir("src/generated/")
        .build_client(true)
        .build_server(true)
        .compile_protos(
            &[
                "client_datanode.proto",
                "datanode_datanode.proto",
                "client_namenode.proto",
                "namenode_datanode.proto",
                "datanode_namenode.proto",
            ],
            &["."],
        )?;
    Ok(())
}
