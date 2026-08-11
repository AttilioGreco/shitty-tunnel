//! Exposed as a library so integration tests in `tests/` can drive the real
//! server and client instead of re-implementing them. The binary in `main.rs`
//! is a thin CLI wrapper over these modules.

pub mod app;
pub mod client_app;
pub mod client_forwarder;
pub mod dashboard;
pub mod public_handler;
pub mod server_main;
pub mod tunnel_handler;
