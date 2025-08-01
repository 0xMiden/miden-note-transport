pub mod generated {
    pub mod account;
    pub mod miden_transport;
    pub mod note;
    pub mod primitives;
}

pub use generated::account;
pub use generated::miden_transport;
pub use generated::note;
pub use generated::primitives;

// Re-export main types
pub use miden_transport::*;

// Re-export miden-node types
pub use account::*;
pub use note::*;
pub use primitives::*;
