pub mod proto {
    tonic::include_proto!("tunnel");
}

pub mod convert;

/// Maximum gRPC message size for tunnel traffic (256 MiB).
/// Applies to both encoding and decoding on client and server.
pub const GRPC_MAX_MESSAGE_SIZE: usize = 256 * 1024 * 1024;
