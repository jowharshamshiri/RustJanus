use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::error::SocketError;

/// Socket command structure (exact SwiftUnixSockAPI parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocketCommand {
    /// Unique identifier for command tracking
    pub id: String,
    
    /// Channel ID for routing
    #[serde(rename = "channelId")]
    pub channel_id: String,
    
    /// Command name
    pub command: String,
    
    /// Command arguments (optional)
    pub args: Option<HashMap<String, serde_json::Value>>,
    
    /// Timeout in seconds (optional)
    pub timeout: Option<f64>,
    
    /// Creation timestamp
    pub timestamp: DateTime<Utc>,
}

impl SocketCommand {
    /// Create a new socket command with UUID
    pub fn new(
        channel_id: String,
        command: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<f64>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            channel_id,
            command,
            args,
            timeout,
            timestamp: Utc::now(),
        }
    }
    
    /// Create command with specific ID (for testing)
    pub fn with_id(
        id: String,
        channel_id: String,
        command: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<f64>,
    ) -> Self {
        Self {
            id,
            channel_id,
            command,
            args,
            timeout,
            timestamp: Utc::now(),
        }
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
    pub fn validate(&self) -> Result<(), SocketError> {
        if self.id.is_empty() {
            return Err(SocketError::ValidationFailed("Command ID cannot be empty".to_string()));
        }
        
        if self.channel_id.is_empty() {
            return Err(SocketError::ValidationFailed("Channel ID cannot be empty".to_string()));
        }
        
        if self.command.is_empty() {
            return Err(SocketError::ValidationFailed("Command name cannot be empty".to_string()));
        }
        
        if let Some(timeout) = self.timeout {
            if timeout <= 0.0 {
                return Err(SocketError::ValidationFailed("Timeout must be positive".to_string()));
            }
        }
        
        Ok(())
    }
}

/// Socket response structure (exact SwiftUnixSockAPI parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocketResponse {
    /// Command ID for correlation
    #[serde(rename = "commandId")]
    pub command_id: String,
    
    /// Channel ID for verification
    #[serde(rename = "channelId")]
    pub channel_id: String,
    
    /// Success/failure flag
    pub success: bool,
    
    /// Response data (optional)
    pub result: Option<serde_json::Value>,
    
    /// Error information (optional)
    pub error: Option<SocketError>,
    
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

impl SocketResponse {
    /// Create successful response
    pub fn success(
        command_id: String,
        channel_id: String,
        result: Option<serde_json::Value>,
    ) -> Self {
        Self {
            command_id,
            channel_id,
            success: true,
            result,
            error: None,
            timestamp: Utc::now(),
        }
    }
    
    /// Create error response
    pub fn error(
        command_id: String,
        channel_id: String,
        error: SocketError,
    ) -> Self {
        Self {
            command_id,
            channel_id,
            success: false,
            result: None,
            error: Some(error),
            timestamp: Utc::now(),
        }
    }
    
    /// Create internal error response
    pub fn internal_error(
        command_id: String,
        channel_id: String,
        message: String,
    ) -> Self {
        Self::error(
            command_id,
            channel_id,
            SocketError::InternalError(message),
        )
    }
    
    /// Create timeout error response
    pub fn timeout_error(
        command_id: String,
        channel_id: String,
        timeout_seconds: f64,
    ) -> Self {
        Self::error(
            command_id.clone(),
            channel_id,
            SocketError::HandlerTimeout(command_id, timeout_seconds),
        )
    }
    
    /// Validate response structure
    pub fn validate(&self) -> Result<(), SocketError> {
        if self.command_id.is_empty() {
            return Err(SocketError::ValidationFailed("Command ID cannot be empty".to_string()));
        }
        
        if self.channel_id.is_empty() {
            return Err(SocketError::ValidationFailed("Channel ID cannot be empty".to_string()));
        }
        
        // If success is true, should not have error
        if self.success && self.error.is_some() {
            return Err(SocketError::ValidationFailed("Successful response cannot have error".to_string()));
        }
        
        // If success is false, should have error
        if !self.success && self.error.is_none() {
            return Err(SocketError::ValidationFailed("Failed response must have error".to_string()));
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

/// Message envelope for framing (exact SwiftUnixSockAPI parity)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocketMessage {
    /// Message type
    pub message_type: MessageType,
    
    /// Encoded payload
    pub payload: Vec<u8>,
}

impl SocketMessage {
    /// Create command message
    pub fn command(command: SocketCommand) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_vec(&command)?;
        Ok(Self {
            message_type: MessageType::Command,
            payload,
        })
    }
    
    /// Create response message
    pub fn response(response: SocketResponse) -> Result<Self, serde_json::Error> {
        let payload = serde_json::to_vec(&response)?;
        Ok(Self {
            message_type: MessageType::Response,
            payload,
        })
    }
    
    /// Decode command from payload
    pub fn decode_command(&self) -> Result<SocketCommand, serde_json::Error> {
        if self.message_type != MessageType::Command {
            return Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message is not a command"
            )));
        }
        serde_json::from_slice(&self.payload)
    }
    
    /// Decode response from payload
    pub fn decode_response(&self) -> Result<SocketResponse, serde_json::Error> {
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
    pub fn validate(&self) -> Result<(), SocketError> {
        if self.payload.is_empty() {
            return Err(SocketError::ValidationFailed("Message payload cannot be empty".to_string()));
        }
        
        // Try to decode based on type to ensure payload is valid
        match self.message_type {
            MessageType::Command => {
                let command = self.decode_command()
                    .map_err(|e| SocketError::ValidationFailed(format!("Invalid command payload: {}", e)))?;
                command.validate()?;
            },
            MessageType::Response => {
                let response = self.decode_response()
                    .map_err(|e| SocketError::ValidationFailed(format!("Invalid response payload: {}", e)))?;
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
        
        let command = SocketCommand::new(
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
        
        let response = SocketResponse::success(
            command_id.to_string(),
            channel_id.to_string(),
            Some(result),
        );
        
        Self::response(response)
    }
    
    /// Create a simple error response
    pub fn simple_error(command_id: &str, channel_id: &str, error_message: &str) -> Result<Self, serde_json::Error> {
        let response = SocketResponse::error(
            command_id.to_string(),
            channel_id.to_string(),
            SocketError::ProcessingError(error_message.to_string()),
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
        
        let command = SocketCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            Some(args),
            Some(30.0),
        );
        
        assert!(!command.id.is_empty());
        assert_eq!(command.channel_id, "test-channel");
        assert_eq!(command.command, "test-command");
        assert!(command.args.is_some());
        assert_eq!(command.timeout, Some(30.0));
        assert!(command.validate().is_ok());
    }
    
    #[test]
    fn test_socket_response_creation() {
        let result = serde_json::json!({"status": "ok"});
        
        let response = SocketResponse::success(
            "test-id".to_string(),
            "test-channel".to_string(),
            Some(result),
        );
        
        assert_eq!(response.command_id, "test-id");
        assert_eq!(response.channel_id, "test-channel");
        assert!(response.success);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
        assert!(response.validate().is_ok());
    }
    
    #[test]
    fn test_socket_message_command() {
        let command = SocketCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            None,
        );
        
        let message = SocketMessage::command(command.clone()).unwrap();
        
        assert_eq!(message.message_type, MessageType::Command);
        assert!(!message.payload.is_empty());
        
        let decoded_command = message.decode_command().unwrap();
        assert_eq!(decoded_command.channel_id, command.channel_id);
        assert_eq!(decoded_command.command, command.command);
    }
    
    #[test]
    fn test_socket_message_response() {
        let response = SocketResponse::success(
            "test-id".to_string(),
            "test-channel".to_string(),
            None,
        );
        
        let message = SocketMessage::response(response.clone()).unwrap();
        
        assert_eq!(message.message_type, MessageType::Response);
        assert!(!message.payload.is_empty());
        
        let decoded_response = message.decode_response().unwrap();
        assert_eq!(decoded_response.command_id, response.command_id);
        assert_eq!(decoded_response.success, response.success);
    }
    
    #[test]
    fn test_message_validation() {
        let command = SocketCommand::new(
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
        let command = SocketCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            Some(30.5),
        );
        
        let duration = command.timeout_duration().unwrap();
        assert_eq!(duration, std::time::Duration::from_secs_f64(30.5));
        
        let command_no_timeout = SocketCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            None,
            None,
        );
        
        assert!(command_no_timeout.timeout_duration().is_none());
    }
}