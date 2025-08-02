// Legacy JanusError enum has been eliminated in favor of JSONRPCError
// All error handling now uses JSON-RPC 2.0 compliant error codes
// See src/error/jsonrpc_error.rs for the standardized error system

pub use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode, JSONRPCErrorData};

impl From<std::io::Error> for JSONRPCError {
    fn from(error: std::io::Error) -> Self {
        JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(error.to_string()))
    }
}

impl From<serde_json::Error> for JSONRPCError {
    fn from(error: serde_json::Error) -> Self {
        JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(error.to_string()))
    }
}

impl From<regex::Error> for JSONRPCError {
    fn from(error: regex::Error) -> Self {
        JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(error.to_string()))
    }
}