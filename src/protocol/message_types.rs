use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use chrono; // PRIME DIRECTIVE: Required for RFC 3339 timestamp format
use crate::error::{JSONRPCError, JSONRPCErrorCode};

/// Socket request structure (PRIME DIRECTIVE: exact cross-language parity with Go/Swift/TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[allow(non_snake_case)]
pub struct JanusRequest {
    /// Unique identifier for request tracking
    pub id: String,
    
    /// Method name being invoked (PRIME DIRECTIVE)
    pub method: String,
    
    /// Request name
    pub request: String,
    
    /// Reply-to socket path for connectionless communication (SOCK_DGRAM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    
    /// Request arguments (optional)
    pub args: Option<HashMap<String, serde_json::Value>>,
    
    /// Timeout in seconds (optional)
    pub timeout: Option<f64>,
    
    /// Creation timestamp (RFC 3339 format)
    pub timestamp: String,
}

impl JanusRequest {
    /// Create a new socket request with UUID
    #[allow(non_snake_case)]
    pub fn new(
        request: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<f64>,
    ) -> Self {
        // PRIME DIRECTIVE: Use RFC 3339 timestamp format
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            method: request.clone(), // PRIME DIRECTIVE: method field matches request name
            request,
            reply_to: None,
            args,
            timeout,
            timestamp,
        }
    }
    
    /// Create request with manifestific ID (for testing)
    #[allow(non_snake_case)]
    pub fn with_id(
        id: String,
        request: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<f64>,
    ) -> Self {
        // PRIME DIRECTIVE: Use RFC 3339 timestamp format
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        Self {
            id,
            method: request.clone(), // PRIME DIRECTIVE: method field matches request name
            request,
            reply_to: None,
            args,
            timeout,
            timestamp,
        }
    }
    
    /// Set reply-to socket path for SOCK_DGRAM response routing
    pub fn with_reply_to(mut self, reply_to: String) -> Self {
        self.reply_to = Some(reply_to);
        self
    }
    
    /// Get timeout as Duration
    pub fn timeout_duration(&self) -> Option<std::time::Duration> {
        self.timeout.map(|t| std::time::Duration::from_secs_f64(t))
    }
    
    /// Check if request has timeout
    pub fn has_timeout(&self) -> bool {
        self.timeout.is_some()
    }
    
    /// Validate request structure
    pub fn validate(&self) -> Result<(), JSONRPCError> {
        if self.id.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Request ID cannot be empty".to_string())));
        }
        
        if self.request.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Request name cannot be empty".to_string())));
        }
        
        if let Some(timeout) = self.timeout {
            if timeout <= 0.0 {
                return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Timeout must be positive".to_string())));
            }
        }
        
        Ok(())
    }
}

/// Socket response structure (PRIME DIRECTIVE: exact format for 100% cross-platform compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(non_snake_case)]
pub struct JanusResponse {
    /// Unwrapped response data
    pub result: Option<serde_json::Value>,
    
    /// Error information (JSON-RPC 2.0 compliant) - null if success
    pub error: Option<JSONRPCError>,
    
    /// Success/failure flag
    pub success: bool,
    
    /// Request ID that this response correlates to
    pub request_id: String,
    
    /// Unique identifier for this response
    pub id: String,
    
    /// Response timestamp (RFC 3339 format)
    pub timestamp: String,
}

impl JanusResponse {
    /// Create successful response (PRIME DIRECTIVE format)
    pub fn success(
        request_id: String,
        result: Option<serde_json::Value>,
    ) -> Self {
        // PRIME DIRECTIVE: Use RFC 3339 timestamp format
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        Self {
            result,
            error: None,
            success: true,
            request_id,
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
        }
    }
    
    /// Create error response with JSON-RPC 2.0 error (PRIME DIRECTIVE format)
    pub fn error(
        request_id: String,
        error: JSONRPCError,
    ) -> Self {
        // PRIME DIRECTIVE: Use RFC 3339 timestamp format
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        Self {
            result: None,
            error: Some(error),
            success: false,
            request_id,
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
        }
    }

    
    /// Create internal error response (PRIME DIRECTIVE format)
    pub fn internal_error(
        request_id: String,
        message: String,
    ) -> Self {
        let jsonrpc_error = JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(message));
        Self::error(request_id, jsonrpc_error)
    }
    
    /// Create timeout error response (PRIME DIRECTIVE format)
    pub fn timeout_error(
        request_id: String,
        timeout_seconds: f64,
    ) -> Self {
        use std::collections::HashMap;
        
        let context = HashMap::from([
            ("requestId".to_string(), serde_json::Value::String(request_id.clone())),
            ("timeoutSeconds".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(timeout_seconds).unwrap())),
        ]);
        
        let jsonrpc_error = JSONRPCError::with_context(
            JSONRPCErrorCode::HandlerTimeout,
            Some(format!("Handler {} timed out after {} seconds", request_id, timeout_seconds)),
            context,
        );
        
        Self::error(request_id, jsonrpc_error)
    }
    
    /// Validate response structure (PRIME DIRECTIVE format)
    pub fn validate(&self) -> Result<(), JSONRPCError> {
        if self.request_id.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Request ID cannot be empty".to_string())));
        }
        
        if self.id.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Response ID cannot be empty".to_string())));
        }
        
        // If success is true, should not have error
        if self.success && self.error.is_some() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Successful response cannot have error".to_string())));
        }
        
        // If success is false, should have error
        if !self.success && self.error.is_none() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Failed response must have error".to_string())));
        }
        
        Ok(())
    }
}

/// Message type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Request,
    Response,
}

/// Message envelope for framing (exact SwiftJanus parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocketMessage {
    /// Message type
    pub message_type: MessageType,
    
    /// Encoded payload
    pub payload: Vec<u8>,
}

/// RequestHandle provides a user-friendly interface to track and manage requests
/// Hides internal UUID complexity from users
#[derive(Debug, Clone)]
pub struct RequestHandle {
    internal_id: String,
    request: String,
    timestamp: SystemTime,
    cancelled: Arc<std::sync::atomic::AtomicBool>,
}

impl RequestHandle {
    /// Create a new request handle from internal UUID
    pub fn new(internal_id: String, request: String) -> Self {
        Self {
            internal_id,
            request,
            timestamp: SystemTime::now(),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    
    /// Get the request name for this request
    pub fn get_request(&self) -> &str {
        &self.request
    }
    
    
    /// Get when this request was created
    pub fn get_timestamp(&self) -> SystemTime {
        self.timestamp
    }
    
    /// Check if this request has been cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Relaxed)
    }
    
    /// Get the internal UUID (for internal use only)
    pub fn get_internal_id(&self) -> &str {
        &self.internal_id
    }
    
    /// Mark this handle as cancelled (internal use only)
    pub fn mark_cancelled(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// RequestStatus represents the status of a tracked request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

impl SocketMessage {
    /// Create request message
    pub fn request(request: JanusRequest) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_vec(&request)?;
        Ok(Self {
            message_type: MessageType::Request,
            payload,
        })
    }
    
    /// Create response message
    pub fn response(response: JanusResponse) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_vec(&response)?;
        Ok(Self {
            message_type: MessageType::Response,
            payload,
        })
    }
    
    /// Decode request from payload
    pub fn decode_request(&self) -> Result<JanusRequest, serde_json::Error> {
        if self.message_type != MessageType::Request {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message is not a request"
            )));
        }
        serde_json::from_slice(&self.payload)
    }
    
    /// Decode response from payload
    pub fn decode_response(&self) -> Result<JanusResponse, serde_json::Error> {
        if self.message_type != MessageType::Response {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message is not a response"
            )));
        }
        serde_json::from_slice(&self.payload)
    }
    
    /// Get payload size
    pub fn payload_size(&self) -> usize {
        self.payload.len()
    }
    
    /// Validate message structure
    pub fn validate(&self) -> Result<(), JSONRPCError> {
        if self.payload.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Message payload cannot be empty".to_string())));
        }
        
        // Try to decode based on type to ensure payload is valid
        match self.message_type {
            MessageType::Request => {
                let request = self.decode_request()
                    .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!("Invalid request payload: {}", e))))?;
                request.validate()?;
            },
            MessageType::Response => {
                let response = self.decode_response()
                    .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!("Invalid response payload: {}", e))))?;
                response.validate()?;
            },
        }
        
        Ok(())
    }
}

/// Utility functions for creating messages
impl SocketMessage {
    /// Create a simple text request
    pub fn text_request(request: &str, text: &str) -> Result<Self, serde_json::Error> {
        let mut args = HashMap::new();
        args.insert("text".to_string(), serde_json::Value::String(text.to_string()));
        
        let request = JanusRequest::new(
            request.to_string(),
            Some(args),
            None,
        );
        
        Self::request(request)
    }
    
    /// Create a simple success response
    pub fn simple_success(request_id: &str, message: &str) -> Result<Self, serde_json::Error> {
        let result = serde_json::json!({
            "message": message
        });
        
        let response = JanusResponse::success(
            request_id.to_string(),
            Some(result),
        );
        
        Self::response(response)
    }
    
    /// Create a simple error response
    pub fn simple_error(request_id: &str, error_message: &str) -> Result<Self, serde_json::Error> {
        use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode};
        
        let jsonrpc_error = JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(error_message.to_string()));
        let response = JanusResponse::error(
            request_id.to_string(),
            jsonrpc_error,
        );
        
        Self::response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_socket_request_creation() {
        let mut args = HashMap::new();
        args.insert("test".to_string(), serde_json::Value::String("value".to_string()));
        
        let request = JanusRequest::new(
            "test-request".to_string(),
            Some(args),
            Some(30.0),
        );
        
        assert!(!request.id.is_empty());
        assert_eq!(request.request, "test-request");
        assert!(request.args.is_some());
        assert_eq!(request.timeout, Some(30.0));
        assert!(request.validate().is_ok());
    }
    
    #[test]
    fn test_socket_response_creation() {
        let result = serde_json::json!({"status": "ok"});
        
        let response = JanusResponse::success(
            "test-id".to_string(),
            Some(result),
        );
        
        assert_eq!(response.request_id, "test-id");
        assert!(response.success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert!(response.validate().is_ok());
    }
    
    #[test]
    fn test_socket_message_request() {
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        let message = SocketMessage::request(request.clone()).unwrap();
        
        assert_eq!(message.message_type, MessageType::Request);
        assert!(!message.payload.is_empty());
        
        let decoded_request = message.decode_request().unwrap();
        assert_eq!(decoded_request.channelId, request.channelId);
        assert_eq!(decoded_request.request, request.request);
    }
    
    #[test]
    fn test_socket_message_response() {
        let response = JanusResponse::success(
            "test-id".to_string(),
            None,
        );
        
        let message = SocketMessage::response(response.clone()).unwrap();
        
        assert_eq!(message.message_type, MessageType::Response);
        assert!(!message.payload.is_empty());
        
        let decoded_response = message.decode_response().unwrap();
        assert_eq!(decoded_response.request_id, response.request_id);
        assert_eq!(decoded_response.success, response.success);
    }
    
    #[test]
    fn test_message_validation() {
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        let message = SocketMessage::request(request).unwrap();
        assert!(message.validate().is_ok());
        
        // Test invalid message
        let invalid_message = SocketMessage {
            message_type: MessageType::Request,
            payload: Vec::new(), // Empty payload
        };
        
        assert!(invalid_message.validate().is_err());
    }
    
    #[test]
    fn test_timeout_duration_conversion() {
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            Some(30.5),
        );
        
        let duration = request.timeout_duration().unwrap();
        assert_eq!(duration, std::time::Duration::from_secs_f64(30.5));
        
        let request_no_timeout = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        assert!(request_no_timeout.timeout_duration().is_none());
    }
}