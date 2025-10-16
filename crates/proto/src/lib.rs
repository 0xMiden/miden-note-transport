//! # Miden Note Transport Protocol Buffers
//!
//! This crate contains the generated Rust bindings for the Miden Note Transport gRPC API.

#[rustfmt::skip]
pub mod generated;

// RE-EXPORTS
// ================================================================================================

// Convenient re-exports for commonly used types
pub mod miden_note_transport {
    pub use super::generated::miden_note_transport::*;
}
