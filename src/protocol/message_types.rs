use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Note: chrono types removed as they are not used in current SOCK_DGRAM implementation
use crate::error::{JSONRPCError, JSONRPCErrorCode};

/// Socket command structure (exact cross-language parity with Go/Swift/TypeScript)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[allow(non_snake_case)]
pub struct JanusCommand {
    /// Unique identifier for command tracking
    pub id: String,
    
    /// Channel ID for routing
    pub channelId: String,
    
    /// Command name
    pub command: String,
    
    /// Reply-to socket path for connectionless communication (SOCK_DGRAM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    
    /// Command arguments (optional)
    pub args: Option<HashMap<String, serde_json::Value>>,
    
    /// Timeout in seconds (optional)
    pub timeout: Option<f64>,
    
    /// Creation timestamp (Unix timestamp as f64)
    pub timestamp: f64,
}

impl JanusCommand {
    /// Create a new socket command with UUID
    #[allow(non_snake_case)]
    pub fn new(
        channelId: String,
        command: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<f64>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channelId,
            command,
            reply_to: None,
            args,
            timeout,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        }
    }
    
    /// Create command with specific ID (for testing)
    #[allow(non_snake_case)]
    pub fn with_id(
        id: String,
        channelId: String,
        command: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<f64>,
    ) -> Self {
        Self {
            id,
            channelId,
            command,
            reply_to: None,
            args,
            timeout,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
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
    
    /// Check if command has timeout
    pub fn has_timeout(&self) -> bool {
        self.timeout.is_some()
    }
    
    /// Validate command structure
    pub fn validate(&self) -> Result<(), JSONRPCError> {
        if self.id.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Command ID cannot be empty".to_string())));
        }
        
        if self.channelId.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Channel ID cannot be empty".to_string())));
        }
        
        if self.command.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Command name cannot be empty".to_string())));
        }
        
        if let Some(timeout) = self.timeout {
            if timeout <= 0.0 {
                return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Timeout must be positive".to_string())));
            }
        }
        
        Ok(())
    }
}

/// Socket response structure (exact SwiftJanus parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(non_snake_case)]
pub struct JanusResponse {
    /// Command ID for correlation
    pub commandId: String,
    
    /// Channel ID for verification
    pub channelId: String,
    
    /// Success/failure flag
    pub success: bool,
    
    /// Response data (optional)
    pub result: Option<serde_json::Value>,
    
    /// Error information (optional) - JSON-RPC 2.0 compliant
    pub error: Option<JSONRPCError>,
    
    /// Response timestamp (Unix timestamp as f64)
    pub timestamp: f64,
}

impl JanusResponse {
    /// Create successful response
    #[allow(non_snake_case)]
    pub fn success(
        commandId: String,
        channelId: String,
        result: Option<serde_json::Value>,
    ) -> Self {
        Self {
            commandId,
            channelId,
            success: true,
            result,
            error: None,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        }
    }
    
    /// Create error response with JSON-RPC 2.0 error
    #[allow(non_snake_case)]
    pub fn error(
        commandId: String,
        channelId: String,
        error: JSONRPCError,
    ) -> Self {
        Self {
            commandId,
            channelId,
            success: false,
            result: None,
            error: Some(error),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        }
    }

    
    /// Create internal error response
    #[allow(non_snake_case)]
    pub fn internal_error(
        commandId: String,
        channelId: String,
        message: String,
    ) -> Self {
        use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode};
        
        let jsonrpc_error = JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(message));
        Self::error(commandId, channelId, jsonrpc_error)
    }
    
    /// Create timeout error response
    #[allow(non_snake_case)]
    pub fn timeout_error(
        commandId: String,
        channelId: String,
        timeout_seconds: f64,
    ) -> Self {
        use crate::error::jsonrpc_error::{JSONRPCError};
        use std::collections::HashMap;
        
        let context = HashMap::from([
            ("commandId".to_string(), serde_json::Value::String(commandId.clone())),
            ("timeoutSeconds".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(timeout_seconds).unwrap())),
        ]);
        
        let jsonrpc_error = JSONRPCError::with_context(
            JSONRPCErrorCode::HandlerTimeout,
            Some(format!("Handler timed out after {} seconds", timeout_seconds)),
            context,
        );
        
        Self::error(commandId, channelId, jsonrpc_error)
    }
    
    /// Validate response structure
    pub fn validate(&self) -> Result<(), JSONRPCError> {
        if self.commandId.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Command ID cannot be empty".to_string())));
        }
        
        if self.channelId.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Channel ID cannot be empty".to_string())));
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
    Command,
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

impl SocketMessage {
    /// Create command message
    pub fn command(command: JanusCommand) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_vec(&command)?;
        Ok(Self {
            message_type: MessageType::Command,
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
    
    /// Decode command from payload
    pub fn decode_command(&self) -> Result<JanusCommand, serde_json::Error> {
        if self.message_type != MessageType::Command {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message is not a command"
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
            MessageType::Command => {
                let command = self.decode_command()
                    .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!("Invalid command payload: {}", e))))?;
                command.validate()?;
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
    /// Create a simple text command
    pub fn text_command(channel_id: &str, command: &str, text: &str) -> Result<Self, serde_json::Error> {
        let mut args = HashMap::new();
        args.insert("text".to_string(), serde_json::Value::String(text.to_string()));
        
        let command = JanusCommand::new(
            channel_id.to_string(),
            command.to_string(),
            Some(args),
            None,
        );
        
        Self::command(command)
    }
    
    /// Create a simple success response
    pub fn simple_success(command_id: &str, channel_id: &str, message: &str) -> Result<Self, serde_json::Error> {
        let result = serde_json::json!({
            "message": message
        });
        
        let response = JanusResponse::success(
            command_id.to_string(),
            channel_id.to_string(),
            Some(result),
        );
        
        Self::response(response)
    }
    
    /// Create a simple error response
    pub fn simple_error(command_id: &str, channel_id: &str, error_message: &str) -> Result<Self, serde_json::Error> {
        use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode};
        
        let jsonrpc_error = JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(error_message.to_string()));
        let response = JanusResponse::error(
            command_id.to_string(),
            channel_id.to_string(),
            jsonrpc_error,
        );
        
        Self::response(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_socket_command_creation() {
        let mut args = HashMap::new();
        args.insert("test".to_string(), serde_json::Value::String("value".to_string()));
        
        let command = JanusCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            Some(args),
            Some(30.0),
        );
        
        assert!(!command.id.is_empty());
        assert_eq!(command.channelId, "test-channel");
        assert_eq!(command.command, "test-command");
        assert!(command.args.is_some());
        assert_eq!(command.timeout, Some(30.0));
        assert!(command.validate().is_ok());
    }
    
    #[test]
    fn test_socket_response_creation() {
        let result = serde_json::json!({"status": "ok"});
        
        let response = JanusResponse::success(
            "test-id".to_string(),
            "test-channel".to_string(),
            Some(result),
        );
        
        assert_eq!(response.commandId, "test-id");
        assert_eq!(response.channelId, "test-channel");
        assert!(response.success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert!(response.validate().is_ok());
    }
    
    #[test]
    fn test_socket_message_command() {
        let command = JanusCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            None,
        );
        
        let message = SocketMessage::command(command.clone()).unwrap();
        
        assert_eq!(message.message_type, MessageType::Command);
        assert!(!message.payload.is_empty());
        
        let decoded_command = message.decode_command().unwrap();
        assert_eq!(decoded_command.channelId, command.channelId);
        assert_eq!(decoded_command.command, command.command);
    }
    
    #[test]
    fn test_socket_message_response() {
        let response = JanusResponse::success(
            "test-id".to_string(),
            "test-channel".to_string(),
            None,
        );
        
        let message = SocketMessage::response(response.clone()).unwrap();
        
        assert_eq!(message.message_type, MessageType::Response);
        assert!(!message.payload.is_empty());
        
        let decoded_response = message.decode_response().unwrap();
        assert_eq!(decoded_response.commandId, response.commandId);
        assert_eq!(decoded_response.success, response.success);
    }
    
    #[test]
    fn test_message_validation() {
        let command = JanusCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            None,
        );
        
        let message = SocketMessage::command(command).unwrap();
        assert!(message.validate().is_ok());
        
        // Test invalid message
        let invalid_message = SocketMessage {
            message_type: MessageType::Command,
            payload: Vec::new(), // Empty payload
        };
        
        assert!(invalid_message.validate().is_err());
    }
    
    #[test]
    fn test_timeout_duration_conversion() {
        let command = JanusCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            Some(30.5),
        );
        
        let duration = command.timeout_duration().unwrap();
        assert_eq!(duration, std::time::Duration::from_secs_f64(30.5));
        
        let command_no_timeout = JanusCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            None,
        );
        
        assert!(command_no_timeout.timeout_duration().is_none());
    }
}