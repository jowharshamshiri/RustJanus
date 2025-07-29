use crate::core::{UnixDatagramClient, SecurityValidator};
use crate::error::UnixSockApiError;
use crate::config::UnixSockApiClientConfig;
use crate::specification::ApiSpecification;
use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Socket command for SOCK_DGRAM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketCommand {
    pub id: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub command: String,
    #[serde(rename = "reply_to", skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
    pub timestamp: String,
}

/// Socket response for SOCK_DGRAM communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketResponse {
    #[serde(rename = "commandId")]
    pub command_id: String,
    #[serde(rename = "channelId")]
    pub channel_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<SocketError>,
    pub timestamp: String,
}

/// Socket error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// High-level API client for SOCK_DGRAM Unix socket communication
/// Connectionless implementation with command validation and response correlation
pub struct UnixSockApiDatagramClient {
    socket_path: String,
    channel_id: String,
    api_spec: Option<ApiSpecification>,
    config: UnixSockApiClientConfig,
    datagram_client: UnixDatagramClient,
    validator: SecurityValidator,
}

impl UnixSockApiDatagramClient {
    /// Create a new datagram API client
    pub fn new(
        socket_path: String,
        channel_id: String,
        api_spec: Option<ApiSpecification>,
        config: UnixSockApiClientConfig,
    ) -> Result<Self, UnixSockApiError> {
        let datagram_client = UnixDatagramClient::new(socket_path.clone(), config.clone())?;
        let validator = SecurityValidator::new();
        
        Ok(Self {
            socket_path,
            channel_id,
            api_spec,
            config,
            datagram_client,
            validator,
        })
    }
    
    /// Send command via SOCK_DGRAM and wait for response
    pub async fn send_command(
        &self,
        command: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<Duration>,
    ) -> Result<SocketResponse, UnixSockApiError> {
        // Generate command ID and response socket path
        let command_id = Uuid::new_v4().to_string();
        let response_socket_path = self.datagram_client.generate_response_socket_path();
        
        // Create socket command
        let socket_command = SocketCommand {
            id: command_id.clone(),
            channel_id: self.channel_id.clone(),
            command: command.to_string(),
            reply_to: Some(response_socket_path.clone()),
            args,
            timeout: timeout.map(|d| d.as_secs_f64()),
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        };
        
        // Validate command against API specification
        if let Some(ref spec) = self.api_spec {
            self.validate_command_against_spec(spec, &socket_command)?;
        }
        
        // Serialize command
        let command_data = serde_json::to_vec(&socket_command)
            .map_err(|e| UnixSockApiError::SerializationError { 
                file: "unix_sock_api_datagram_client.rs".to_string(), 
                line: 111, 
                message: format!("Failed to serialize command: {}", e) 
            })?;
        
        // Send datagram and wait for response
        let response_data = self.datagram_client
            .send_datagram(&command_data, &response_socket_path)
            .await?;
        
        // Deserialize response
        let response: SocketResponse = serde_json::from_slice(&response_data)
            .map_err(|e| UnixSockApiError::SerializationError { 
                file: "unix_sock_api_datagram_client.rs".to_string(), 
                line: 124, 
                message: format!("Failed to deserialize response: {}", e) 
            })?;
        
        // Validate response correlation
        if response.command_id != command_id {
            return Err(UnixSockApiError::ProtocolError { 
                file: "unix_sock_api_datagram_client.rs".to_string(), 
                line: 129, 
                message: format!(
                    "Response correlation mismatch: expected {}, got {}",
                    command_id, response.command_id
                ) 
            });
        }
        
        if response.channel_id != self.channel_id {
            return Err(UnixSockApiError::ProtocolError { 
                file: "unix_sock_api_datagram_client.rs".to_string(), 
                line: 136, 
                message: format!(
                    "Channel mismatch: expected {}, got {}",
                    self.channel_id, response.channel_id
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
    ) -> Result<(), UnixSockApiError> {
        // Generate command ID
        let command_id = Uuid::new_v4().to_string();
        
        // Create socket command (no reply_to field)
        let socket_command = SocketCommand {
            id: command_id,
            channel_id: self.channel_id.clone(),
            command: command.to_string(),
            reply_to: None,
            args,
            timeout: None,
            timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        };
        
        // Validate command against API specification
        if let Some(ref spec) = self.api_spec {
            self.validate_command_against_spec(spec, &socket_command)?;
        }
        
        // Serialize command
        let command_data = serde_json::to_vec(&socket_command)
            .map_err(|e| UnixSockApiError::SerializationError { 
                file: "unix_sock_api_datagram_client.rs".to_string(), 
                line: 167, 
                message: format!("Failed to serialize command: {}", e) 
            })?;
        
        // Send datagram without waiting for response
        self.datagram_client.send_datagram_no_response(&command_data).await?;
        
        Ok(())
    }
    
    /// Test connectivity to the server
    pub async fn test_connection(&self) -> Result<(), UnixSockApiError> {
        self.datagram_client.test_connection().await
    }
    
    /// Validate command against API specification
    fn validate_command_against_spec(
        &self,
        spec: &ApiSpecification,
        command: &SocketCommand,
    ) -> Result<(), UnixSockApiError> {
        // Implementation would validate command against spec
        // For now, just check if channel exists
        if !spec.channels.contains_key(&command.channel_id) {
            return Err(UnixSockApiError::ValidationError(format!(
                "Channel {} not found in API specification",
                command.channel_id
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
}