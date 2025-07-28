use crate::specification::ApiSpecification;
use crate::config::UnixSockApiClientConfig;
use crate::error::{UnixSockApiError, Result};
use crate::protocol::{SocketCommand, SocketResponse, TimeoutHandler};
use crate::core::SecurityValidator;
use std::collections::HashMap;
use std::time::Duration;

/// Main UnixSockApiClient (exact SwiftUnixSockAPI parity)
#[derive(Debug)]
pub struct UnixSockApiClient {
    socket_path: String,
    channel_id: String,
    api_spec: ApiSpecification,
    config: UnixSockApiClientConfig,
}

impl UnixSockApiClient {
    /// Create a new UnixSockApiClient
    pub async fn new(
        socket_path: String,
        channel_id: String,
        api_spec: ApiSpecification,
        config: UnixSockApiClientConfig,
    ) -> Result<Self> {
        // Validate socket path
        SecurityValidator::validate_socket_path(&socket_path)?;
        
        // Validate channel ID
        SecurityValidator::validate_channel_id(&channel_id, &config)?;
        
        // Validate configuration
        config.validate()
            .map_err(|e| UnixSockApiError::ValidationError(e))?;
        
        Ok(Self {
            socket_path,
            channel_id,
            api_spec,
            config,
        })
    }
    
    /// Send a command with timeout
    pub async fn send_command(
        &self,
        command_name: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Duration,
        _on_timeout: Option<TimeoutHandler>,
    ) -> Result<SocketResponse> {
        // Validate command name
        SecurityValidator::validate_command_name(command_name, &self.config)?;
        
        // Validate arguments size
        SecurityValidator::validate_args_size(&args, &self.config)?;
        
        // Create command
        let command = SocketCommand::new(
            self.channel_id.clone(),
            command_name.to_string(),
            args,
            Some(timeout.as_secs_f64()),
        );
        
        // For now, return a mock response
        // TODO: Implement actual command execution
        Ok(SocketResponse::success(
            command.id,
            self.channel_id.clone(),
            Some(serde_json::json!({"mock": "response"})),
        ))
    }
    
    /// Publish a command without waiting for response
    pub async fn publish_command(
        &self,
        command_name: &str,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String> {
        // Validate command name
        SecurityValidator::validate_command_name(command_name, &self.config)?;
        
        // Validate arguments size
        SecurityValidator::validate_args_size(&args, &self.config)?;
        
        // Create command
        let command = SocketCommand::new(
            self.channel_id.clone(),
            command_name.to_string(),
            args,
            None,
        );
        
        // TODO: Implement actual command publishing
        Ok(command.id)
    }
    
    /// Register a command handler
    pub async fn register_command_handler(
        &self,
        _command_name: &str,
        _handler: crate::protocol::CommandHandler,
    ) -> Result<()> {
        // TODO: Implement handler registration
        Ok(())
    }
    
    /// Start listening for commands
    pub async fn start_listening(&self) -> Result<()> {
        // TODO: Implement persistent listening
        Ok(())
    }
    
    /// Get configuration
    pub fn configuration(&self) -> &UnixSockApiClientConfig {
        &self.config
    }
    
    /// Get specification
    pub fn specification(&self) -> &ApiSpecification {
        &self.api_spec
    }
}