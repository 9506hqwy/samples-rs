use std::error::Error;
use tonic_prost_build::configure;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed=../proto/service.proto");

    configure()
        .build_server(false)
        .compile_protos(&["../proto/service.proto"], &["../proto"])?;
    Ok(())
}
