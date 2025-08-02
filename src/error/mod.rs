pub mod api_error;
pub mod jsonrpc_error;

pub use api_error::JanusError;
pub use jsonrpc_error::{JSONRPCError, JSONRPCErrorCode, JSONRPCErrorData};

pub type Result<T> = std::result::Result<T, JanusError>;