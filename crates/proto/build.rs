use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-env-changed=BUILD_PROTO");

    // Only generate protobuf files if BUILD_PROTO environment variable is set to "1"
    if env::var("BUILD_PROTO").unwrap_or_default() != "1" {
        println!("cargo:warning=Skipping protobuf generation");
        return Ok(());
    }

    let generated_dir = "src/generated";

    // Create the generated directory if it doesn't exist
    std::fs::create_dir_all(generated_dir)?;

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(generated_dir)
        .compile_protos(
            &["../proto/transport.proto"],
            &["../proto", "../proto/miden-node/proto/proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/transport.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/note.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/account.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/primitives.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/blockchain.proto");

    Ok(())
}
