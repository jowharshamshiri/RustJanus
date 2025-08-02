pub mod api_error;
pub mod jsonrpc_error;

// Legacy JanusError eliminated - use JSONRPCError for all error handling
pub use jsonrpc_error::{JSONRPCError, JSONRPCErrorCode, JSONRPCErrorData};

pub type Result<T> = std::result::Result<T, JSONRPCError>;