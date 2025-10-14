//! # Miden Private Notes Transport Protocol Buffers
//!
//! This crate contains the generated Rust bindings for the Miden Private Notes Transport gRPC API.

#[rustfmt::skip]
pub mod generated;

// RE-EXPORTS
// ================================================================================================

// Convenient re-exports for commonly used types
pub mod miden_private_transport {
    pub use super::generated::miden_private_transport::*;
}
