use crate::core::{JanusClient, SecurityValidator};
use crate::error::JanusError;
use crate::config::JanusClientConfig;
use crate::specification::ApiSpecification;
use crate::protocol::message_types::{SocketCommand, SocketResponse};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// Note: SocketCommand, SocketResponse, and SocketError types are imported from message_types module
// This ensures cross-language parity and eliminates type duplication

/// High-level API client for SOCK_DGRAM Unix socket communication
/// Connectionless implementation with command validation and response correlation
#[derive(Debug)]
pub struct JanusClient {
    socket_path: String,
    channel_id: String,
    api_spec: Option<ApiSpecification>,
    config: JanusClientConfig,
    janus_client: JanusClient,
    // Note: SecurityValidator is used via static methods, no instance needed
}

impl JanusClient {
    /// Create a new datagram API client
    pub fn new(
        socket_path: String,
        channel_id: String,
        api_spec: Option<ApiSpecification>,
        config: JanusClientConfig,
    ) -> Result<Self, JanusError> {
        let janus_client = JanusClient::new(socket_path.clone(), config.clone())?;
        
        Ok(Self {
            socket_path,
            channel_id,
            api_spec,
            config,
            janus_client,
        })
    }
    
    /// Send command via SOCK_DGRAM and wait for response
    pub async fn send_command(
        &self,
        command: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<Duration>,
    ) -> Result<SocketResponse, JanusError> {
        // Generate command ID and response socket path
        let command_id = Uuid::new_v4().to_string();
        let response_socket_path = self.janus_client.generate_response_socket_path();
        
        // Create socket command
        let socket_command = SocketCommand {
            id: command_id.clone(),
            channelId: self.channel_id.clone(),
            command: command.to_string(),
            reply_to: Some(response_socket_path.clone()),
            args,
            timeout: timeout.map(|d| d.as_secs_f64()),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        };
        
        // Validate command against API specification
        if let Some(ref spec) = self.api_spec {
            self.validate_command_against_spec(spec, &socket_command)?;
        }
        
        // Apply security validation
        SecurityValidator::validate_socket_path(&response_socket_path)?;
        
        // Serialize command
        let command_data = serde_json::to_vec(&socket_command)
            .map_err(|e| JanusError::SerializationError { 
                file: "janus_client.rs".to_string(), 
                line: 111, 
                message: format!("Failed to serialize command: {}", e) 
            })?;
        
        // Send datagram and wait for response
        let response_data = self.janus_client
            .send_datagram(&command_data, &response_socket_path)
            .await?;
        
        // Deserialize response
        let response: SocketResponse = serde_json::from_slice(&response_data)
            .map_err(|e| JanusError::SerializationError { 
                file: "janus_client.rs".to_string(), 
                line: 124, 
                message: format!("Failed to deserialize response: {}", e) 
            })?;
        
        // Validate response correlation
        if response.commandId != command_id {
            return Err(JanusError::ProtocolError { 
                file: "janus_client.rs".to_string(), 
                line: 129, 
                message: format!(
                    "Response correlation mismatch: expected {}, got {}",
                    command_id, response.commandId
                ) 
            });
        }
        
        if response.channelId != self.channel_id {
            return Err(JanusError::ProtocolError { 
                file: "janus_client.rs".to_string(), 
                line: 136, 
                message: format!(
                    "Channel mismatch: expected {}, got {}",
                    self.channel_id, response.channelId
                ) 
            });
        }
        
        Ok(response)
    }
    
    /// Send command without expecting response (fire-and-forget)
    pub async fn send_command_no_response(
        &self,
        command: &str,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<(), JanusError> {
        // Generate command ID
        let command_id = Uuid::new_v4().to_string();
        
        // Create socket command (no reply_to field)
        let socket_command = SocketCommand {
            id: command_id,
            channelId: self.channel_id.clone(),
            command: command.to_string(),
            reply_to: None,
            args,
            timeout: None,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        };
        
        // Validate command against API specification
        if let Some(ref spec) = self.api_spec {
            self.validate_command_against_spec(spec, &socket_command)?;
        }
        
        // Serialize command
        let command_data = serde_json::to_vec(&socket_command)
            .map_err(|e| JanusError::SerializationError { 
                file: "janus_client.rs".to_string(), 
                line: 167, 
                message: format!("Failed to serialize command: {}", e) 
            })?;
        
        // Send datagram without waiting for response
        self.janus_client.send_datagram_no_response(&command_data).await?;
        
        Ok(())
    }
    
    /// Test connectivity to the server
    pub async fn test_connection(&self) -> Result<(), JanusError> {
        self.janus_client.test_connection().await
    }
    
    /// Validate command against API specification
    fn validate_command_against_spec(
        &self,
        spec: &ApiSpecification,
        command: &SocketCommand,
    ) -> Result<(), JanusError> {
        // Implementation would validate command against spec
        // For now, just check if channel exists
        if !spec.channels.contains_key(&command.channelId) {
            return Err(JanusError::ValidationError(format!(
                "Channel {} not found in API specification",
                command.channelId
            )));
        }
        
        Ok(())
    }
    
    /// Get channel ID
    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }
    
    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    /// Get API specification
    pub fn api_specification(&self) -> Option<&ApiSpecification> {
        self.api_spec.as_ref()
    }
    
    /// Get configuration for backward compatibility
    pub fn configuration(&self) -> &JanusClientConfig {
        &self.config
    }
    
    /// Get specification for backward compatibility  
    pub fn specification(&self) -> Option<&ApiSpecification> {
        self.api_spec.as_ref()
    }
    
    /// Send a ping command and return success/failure
    /// Convenience method for testing connectivity with a simple ping command
    pub async fn ping(&self) -> bool {
        match self.send_command("ping", None, None).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}