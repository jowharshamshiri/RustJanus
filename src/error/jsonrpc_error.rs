use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// JSON-RPC 2.0 compliant error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum JSONRPCErrorCode {
    // Standard JSON-RPC 2.0 error codes
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,

    // Implementation-defined server error codes (-32000 to -32099)
    ServerError = -32000,
    ServiceUnavailable = -32001,
    AuthenticationFailed = -32002,
    RateLimitExceeded = -32003,
    ResourceNotFound = -32004,
    ValidationFailed = -32005,
    HandlerTimeout = -32006,
    SocketError = -32007,
    ConfigurationError = -32008,
    SecurityViolation = -32009,
    ResourceLimitExceeded = -32010,
}

impl JSONRPCErrorCode {
    /// Returns the string representation of the error code
    pub fn as_str(&self) -> &'static str {
        match self {
            JSONRPCErrorCode::ParseError => "PARSE_ERROR",
            JSONRPCErrorCode::InvalidRequest => "INVALID_REQUEST",
            JSONRPCErrorCode::MethodNotFound => "METHOD_NOT_FOUND",
            JSONRPCErrorCode::InvalidParams => "INVALID_PARAMS",
            JSONRPCErrorCode::InternalError => "INTERNAL_ERROR",
            JSONRPCErrorCode::ServerError => "SERVER_ERROR",
            JSONRPCErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            JSONRPCErrorCode::AuthenticationFailed => "AUTHENTICATION_FAILED",
            JSONRPCErrorCode::RateLimitExceeded => "RATE_LIMIT_EXCEEDED",
            JSONRPCErrorCode::ResourceNotFound => "RESOURCE_NOT_FOUND",
            JSONRPCErrorCode::ValidationFailed => "VALIDATION_FAILED",
            JSONRPCErrorCode::HandlerTimeout => "HANDLER_TIMEOUT",
            JSONRPCErrorCode::SocketError => "SOCKET_ERROR",
            JSONRPCErrorCode::ConfigurationError => "CONFIGURATION_ERROR",
            JSONRPCErrorCode::SecurityViolation => "SECURITY_VIOLATION",
            JSONRPCErrorCode::ResourceLimitExceeded => "RESOURCE_LIMIT_EXCEEDED",
        }
    }

    /// Returns the standard human-readable message for the error code
    pub fn message(&self) -> &'static str {
        match self {
            JSONRPCErrorCode::ParseError => "Parse error",
            JSONRPCErrorCode::InvalidRequest => "Invalid Request",
            JSONRPCErrorCode::MethodNotFound => "Method not found",
            JSONRPCErrorCode::InvalidParams => "Invalid params",
            JSONRPCErrorCode::InternalError => "Internal error",
            JSONRPCErrorCode::ServerError => "Server error",
            JSONRPCErrorCode::ServiceUnavailable => "Service unavailable",
            JSONRPCErrorCode::AuthenticationFailed => "Authentication failed",
            JSONRPCErrorCode::RateLimitExceeded => "Rate limit exceeded",
            JSONRPCErrorCode::ResourceNotFound => "Resource not found",
            JSONRPCErrorCode::ValidationFailed => "Validation failed",
            JSONRPCErrorCode::HandlerTimeout => "Handler timeout",
            JSONRPCErrorCode::SocketError => "Socket error",
            JSONRPCErrorCode::ConfigurationError => "Configuration error",
            JSONRPCErrorCode::SecurityViolation => "Security violation",
            JSONRPCErrorCode::ResourceLimitExceeded => "Resource limit exceeded",
        }
    }

    /// Returns the numeric error code value
    pub fn code(&self) -> i32 {
        *self as i32
    }
}

impl fmt::Display for JSONRPCErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Additional error context information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JSONRPCErrorData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<HashMap<String, serde_json::Value>>,
}

impl JSONRPCErrorData {
    /// Creates a new empty error data
    pub fn new() -> Self {
        Self {
            details: None,
            field: None,
            value: None,
            constraints: None,
            context: None,
        }
    }

    /// Creates error data with just details
    pub fn with_details<S: Into<String>>(details: S) -> Self {
        Self {
            details: Some(details.into()),
            field: None,
            value: None,
            constraints: None,
            context: None,
        }
    }

    /// Creates error data with validation information
    pub fn with_validation<S: Into<String>>(
        field: S,
        value: serde_json::Value,
        details: S,
    ) -> Self {
        Self {
            details: Some(details.into()),
            field: Some(field.into()),
            value: Some(value),
            constraints: None,
            context: None,
        }
    }

    /// Adds context information
    pub fn with_context(mut self, context: HashMap<String, serde_json::Value>) -> Self {
        self.context = Some(context);
        self
    }

    /// Adds constraints information
    pub fn with_constraints(mut self, constraints: HashMap<String, serde_json::Value>) -> Self {
        self.constraints = Some(constraints);
        self
    }
}

impl Default for JSONRPCErrorData {
    fn default() -> Self {
        Self::new()
    }
}

/// JSON-RPC 2.0 compliant error structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JSONRPCError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<JSONRPCErrorData>,
}

impl JSONRPCError {
    /// Creates a new JSON-RPC error with the specified code
    pub fn new(code: JSONRPCErrorCode, details: Option<String>) -> Self {
        let data = details.map(JSONRPCErrorData::with_details);
        
        Self {
            code: code.code(),
            message: code.message().to_string(),
            data,
        }
    }

    /// Creates a new JSON-RPC error with additional context
    pub fn with_context(
        code: JSONRPCErrorCode,
        details: Option<String>,
        context: HashMap<String, serde_json::Value>,
    ) -> Self {
        let data = Some(
            JSONRPCErrorData::with_details(details.unwrap_or_default())
                .with_context(context)
        );
        
        Self {
            code: code.code(),
            message: code.message().to_string(),
            data,
        }
    }

    /// Creates a validation-specific JSON-RPC error
    pub fn validation_error<S: Into<String>>(
        field: S,
        value: serde_json::Value,
        details: S,
        constraints: Option<HashMap<String, serde_json::Value>>,
    ) -> Self {
        let mut data = JSONRPCErrorData::with_validation(field, value, details);
        if let Some(constraints) = constraints {
            data = data.with_constraints(constraints);
        }
        
        Self {
            code: JSONRPCErrorCode::ValidationFailed.code(),
            message: JSONRPCErrorCode::ValidationFailed.message().to_string(),
            data: Some(data),
        }
    }

    /// Returns the error code as an enum if it's a known code
    pub fn error_code(&self) -> Option<JSONRPCErrorCode> {
        match self.code {
            -32700 => Some(JSONRPCErrorCode::ParseError),
            -32600 => Some(JSONRPCErrorCode::InvalidRequest),
            -32601 => Some(JSONRPCErrorCode::MethodNotFound),
            -32602 => Some(JSONRPCErrorCode::InvalidParams),
            -32603 => Some(JSONRPCErrorCode::InternalError),
            -32000 => Some(JSONRPCErrorCode::ServerError),
            -32001 => Some(JSONRPCErrorCode::ServiceUnavailable),
            -32002 => Some(JSONRPCErrorCode::AuthenticationFailed),
            -32003 => Some(JSONRPCErrorCode::RateLimitExceeded),
            -32004 => Some(JSONRPCErrorCode::ResourceNotFound),
            -32005 => Some(JSONRPCErrorCode::ValidationFailed),
            -32006 => Some(JSONRPCErrorCode::HandlerTimeout),
            -32007 => Some(JSONRPCErrorCode::SocketError),
            -32008 => Some(JSONRPCErrorCode::ConfigurationError),
            -32009 => Some(JSONRPCErrorCode::SecurityViolation),
            -32010 => Some(JSONRPCErrorCode::ResourceLimitExceeded),
            _ => None,
        }
    }
}

impl fmt::Display for JSONRPCError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(data) = &self.data {
            if let Some(details) = &data.details {
                return write!(f, "JSON-RPC Error {}: {} - {}", self.code, self.message, details);
            }
        }
        write!(f, "JSON-RPC Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JSONRPCError {}


/// Legacy JanusError mapping for backward compatibility  
pub fn map_legacy_janus_error(legacy_error: &crate::error::api_error::JanusError) -> JSONRPCErrorCode {
    use crate::error::api_error::JanusError;
    
    match legacy_error {
        JanusError::UnknownCommand(_) => JSONRPCErrorCode::MethodNotFound,
        JanusError::InvalidArgument(_, _) => JSONRPCErrorCode::InvalidParams,
        JanusError::HandlerTimeout(_, _) => JSONRPCErrorCode::HandlerTimeout,
        JanusError::SecurityViolation(_) => JSONRPCErrorCode::SecurityViolation,
        JanusError::ResourceLimit(_) => JSONRPCErrorCode::ResourceLimitExceeded,
        _ => JSONRPCErrorCode::InternalError,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        assert_eq!(JSONRPCErrorCode::ParseError.code(), -32700);
        assert_eq!(JSONRPCErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(JSONRPCErrorCode::ValidationFailed.code(), -32005);
    }

    #[test]
    fn test_error_code_messages() {
        assert_eq!(JSONRPCErrorCode::ParseError.message(), "Parse error");
        assert_eq!(JSONRPCErrorCode::MethodNotFound.message(), "Method not found");
        assert_eq!(JSONRPCErrorCode::ValidationFailed.message(), "Validation failed");
    }

    #[test]
    fn test_jsonrpc_error_creation() {
        let error = JSONRPCError::new(JSONRPCErrorCode::MethodNotFound, Some("Command not found".to_string()));
        
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert!(error.data.is_some());
        assert_eq!(error.data.unwrap().details, Some("Command not found".to_string()));
    }

    #[test]
    fn test_validation_error_creation() {
        let constraints = HashMap::from([
            ("min".to_string(), serde_json::Value::Number(serde_json::Number::from(1))),
            ("max".to_string(), serde_json::Value::Number(serde_json::Number::from(10))),
        ]);
        
        let error = JSONRPCError::validation_error(
            "age",
            serde_json::Value::Number(serde_json::Number::from(0)),
            "Value must be between 1 and 10",
            Some(constraints.clone()),
        );
        
        assert_eq!(error.code, -32005);
        assert_eq!(error.message, "Validation failed");
        
        let data = error.data.unwrap();
        assert_eq!(data.field, Some("age".to_string()));
        assert_eq!(data.value, Some(serde_json::Value::Number(serde_json::Number::from(0))));
        assert_eq!(data.details, Some("Value must be between 1 and 10".to_string()));
        assert_eq!(data.constraints, Some(constraints));
    }

    #[test]
    fn test_json_serialization() {
        let error = JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some("Missing required parameter".to_string()));
        let json = serde_json::to_string(&error).unwrap();
        
        assert!(json.contains("\"code\":-32602"));
        assert!(json.contains("\"message\":\"Invalid params\""));
        assert!(json.contains("\"details\":\"Missing required parameter\""));
    }

    #[test]
    fn test_json_deserialization() {
        let json = r#"{"code":-32601,"message":"Method not found","data":{"details":"Command 'test' not found"}}"#;
        let error: JSONRPCError = serde_json::from_str(json).unwrap();
        
        assert_eq!(error.code, -32601);
        assert_eq!(error.message, "Method not found");
        assert_eq!(error.error_code(), Some(JSONRPCErrorCode::MethodNotFound));
        
        let data = error.data.unwrap();
        assert_eq!(data.details, Some("Command 'test' not found".to_string()));
    }
}