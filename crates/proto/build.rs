use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-env-changed=BUILD_PROTO");

    // Only generate protobuf files if BUILD_PROTO environment variable is set to "1"
    if env::var("BUILD_PROTO").unwrap_or_default() != "1" {
        println!("cargo:warning=Skipping protobuf generation");
        return Ok(());
    }

    let generated_transport_dir = "src/generated_transport";
    let generated_dir = "src/generated";

    // Create the generated directory if it doesn't exist
    std::fs::create_dir_all(generated_dir)?;

    // generate with 'transport'
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .build_transport(true)
        .out_dir(generated_transport_dir)
        .compile_protos(
            &["../proto/miden-private-transport.proto"],
            &["../proto", "../proto/miden-node/proto/proto"],
        )?;

    // generate without 'transport'
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .build_transport(false)
        .out_dir(generated_dir)
        .compile_protos(
            &["../proto/miden-private-transport.proto"],
            &["../proto", "../proto/miden-node/proto/proto"],
        )?;

    println!("cargo:rerun-if-changed=../proto/miden-private-transport.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/note.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/account.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/primitives.proto");
    println!("cargo:rerun-if-changed=../proto/miden-node/proto/proto/types/blockchain.proto");

    Ok(())
}
