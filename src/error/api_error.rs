use thiserror::Error;
use std::time::Duration;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum UnixSockApiError {
    #[error("Invalid channel: {0}")]
    InvalidChannel(String),
    
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    
    #[error("Missing required argument: {0}")]
    MissingRequiredArgument(String),
    
    #[error("Invalid argument '{0}': {1}")]
    InvalidArgument(String, String),
    
    #[error("Connection required for operation")]
    ConnectionRequired,
    
    #[error("Encoding failed: {0}")]
    EncodingFailed(String),
    
    #[error("Decoding failed: {0}")]
    DecodingFailed(String),
    
    #[error("Command '{0}' timed out after {1:?}")]
    CommandTimeout(String, Duration),
    
    #[error("Handler for command '{0}' timed out after {1:?}")]
    HandlerTimeout(String, Duration),
    
    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),
    
    #[error("Invalid socket path: {0}")]
    InvalidSocketPath(String),
    
    #[error("Security violation: {0}")]
    SecurityViolation(String),
    
    #[error("Malformed data: {0}")]
    MalformedData(String),
    
    #[error("Message too large: {0} bytes (limit: {1} bytes)")]
    MessageTooLarge(usize, usize),
    
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Serialization error in {file}:{line}: {message}")]
    SerializationError { file: String, line: u32, message: String },
    
    #[error("Protocol error in {file}:{line}: {message}")]
    ProtocolError { file: String, line: u32, message: String },
    
    #[error("Specification error in {file}:{line}: {message}")]
    SpecificationError { file: String, line: u32, message: String },
}

impl From<std::io::Error> for UnixSockApiError {
    fn from(error: std::io::Error) -> Self {
        UnixSockApiError::IoError(error.to_string())
    }
}

impl From<serde_json::Error> for UnixSockApiError {
    fn from(error: serde_json::Error) -> Self {
        UnixSockApiError::DecodingFailed(error.to_string())
    }
}

impl From<regex::Error> for UnixSockApiError {
    fn from(error: regex::Error) -> Self {
        UnixSockApiError::ValidationError(error.to_string())
    }
}