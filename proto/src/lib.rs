use protox::prost::Message;
use tonic_prost_build::FileDescriptorSet;

/// Returns the Protobuf file descriptor for the Miden Note Transport API.
pub fn mnt_api_descriptor() -> FileDescriptorSet {
    let bytes =
        include_bytes!(concat!(env!("OUT_DIR"), "/", "miden_note_transport_file_descriptor.bin"));
    FileDescriptorSet::decode(&bytes[..])
        .expect("bytes should be a valid file descriptor created by build.rs")
}
