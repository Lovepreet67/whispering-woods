use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_build::configure()
        .out_dir("src/generated/")
        .build_client(true)
        .build_server(true)
        .compile_protos(&["data.proto"], &["."])?;
    Ok(())
}
