use std::env;
use std::path::PathBuf;

use fs_err as fs;
use miette::{Context, IntoDiagnostic};
use protox::prost::Message;

const MNT_PROTO: &str = "miden_note_transport.proto";
const MNT_DESCRIPTOR: &str = "miden_note_transport_file_descriptor.bin";

/// Generates Rust protobuf bindings from .proto files.
///
/// This is done only if `BUILD_PROTO` environment variable is set to `1` to avoid running the
/// script on crates.io where repo-level .proto files are not available.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo::rerun-if-changed=./proto");
    println!("cargo::rerun-if-env-changed=BUILD_PROTO");

    let out =
        env::var("OUT_DIR").expect("env::OUT_DIR is always set in build.rs when used with cargo");

    let crate_root: PathBuf = env!("CARGO_MANIFEST_DIR").into();
    let proto_dir = crate_root.join("proto");
    let includes = &[proto_dir];

    let mnt_file_descriptor = protox::compile([MNT_PROTO], includes)?;
    let mnt_path = PathBuf::from(&out).join(MNT_DESCRIPTOR);
    fs::write(&mnt_path, mnt_file_descriptor.encode_to_vec())
        .into_diagnostic()
        .wrap_err("writing mnt file descriptor")?;

    Ok(())
}
