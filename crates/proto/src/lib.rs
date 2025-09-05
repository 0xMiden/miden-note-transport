#[allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#[cfg(feature = "transport")]
pub mod generated_transport {
    pub mod account;
    pub mod miden_private_transport;
    pub mod note;
    pub mod primitives;
}
// Re-export
#[cfg(feature = "transport")]
pub use {
    account::*,
    generated_transport::{account, miden_private_transport, note, primitives},
    miden_private_transport::*,
    note::*,
    primitives::*,
};

#[allow(clippy::all, clippy::pedantic, clippy::restriction, clippy::nursery)]
#[cfg(not(feature = "transport"))]
pub mod generated {
    pub mod account;
    pub mod miden_private_transport;
    pub mod note;
    pub mod primitives;
}
// Re-export
#[cfg(not(feature = "transport"))]
pub use {
    account::*,
    generated::{account, miden_private_transport, note, primitives},
    miden_private_transport::*,
    note::*,
    primitives::*,
};
