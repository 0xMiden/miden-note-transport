pub mod generated {
    pub mod account;
    pub mod miden_transport;
    pub mod note;
    pub mod primitives;
}

// Re-export miden-node types
pub use account::*;
pub use generated::{account, miden_transport, note, primitives};
// Re-export main types
pub use miden_transport::*;
pub use note::*;
pub use primitives::*;
