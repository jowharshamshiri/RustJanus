use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SocketError {
    #[error("Invalid argument '{0}': {1}")]
    InvalidArgument(String, String),
    
    #[error("Command handler timed out for '{0}' after {1} seconds")]
    HandlerTimeout(String, f64),  // Duration as f64 seconds for JSON compatibility
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Processing error: {0}")]
    ProcessingError(String),
    
    #[error("Internal error: {0}")]
    InternalError(String),
    
    #[error("Missing required field: {0}")]
    MissingRequired(String),
    
    #[error("Type mismatch for field '{0}': expected {1}, got {2}")]
    TypeMismatch(String, String, String),
    
    #[error("Value out of range for field '{0}': {1}")]
    ValueOutOfRange(String, String),
    
    #[error("Pattern mismatch for field '{0}': {1}")]
    PatternMismatch(String, String),
    
    #[error("Command not found: {0}")]
    CommandNotFound(String),
}