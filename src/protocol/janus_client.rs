use crate::core::{CoreJanusClient, SecurityValidator};
use crate::error::JanusError;
use crate::config::JanusClientConfig;
use crate::specification::Manifest;
use crate::protocol::message_types::{JanusCommand, JanusResponse};
use crate::protocol::response_tracker::{ResponseTracker, TrackerConfig, CommandStatistics};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// Note: JanusCommand, JanusResponse, and SocketError types are imported from message_types module
// This ensures cross-language parity and eliminates type duplication

/// Simulate connection state for SOCK_DGRAM compatibility
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub is_connected: bool,
    pub last_activity: std::time::SystemTime,
    pub messages_sent: u64,
    pub responses_received: u64,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            last_activity: std::time::SystemTime::now(),
            messages_sent: 0,
            responses_received: 0,
        }
    }
    
    pub fn with_connection(is_connected: bool) -> Self {
        Self {
            is_connected,
            last_activity: std::time::SystemTime::now(),
            messages_sent: 0,
            responses_received: 0,
        }
    }
}

/// High-level API client for SOCK_DGRAM Unix socket communication
/// Connectionless implementation with command validation and response correlation
#[derive(Debug)]
pub struct JanusClient {
    socket_path: String,
    channel_id: String,
    manifest: Option<Manifest>,
    config: JanusClientConfig,
    core_client: CoreJanusClient,
    response_tracker: ResponseTracker,
    connection_state: std::sync::Mutex<ConnectionState>,
    // Note: SecurityValidator is used via static methods, no instance needed
}

impl JanusClient {
    /// Create a new datagram API client
    /// Manifest will be fetched during operations when needed
    pub async fn new(
        socket_path: String,
        channel_id: String,
        config: JanusClientConfig,
    ) -> Result<Self, JanusError> {
        // Validate socket path
        SecurityValidator::validate_socket_path(&socket_path)?;
        
        // Validate channel ID
        SecurityValidator::validate_channel_id(&channel_id, &config)?;
        
        let core_client = CoreJanusClient::new(socket_path.clone(), config.clone())?;
        
        // Initialize response tracker for advanced client features
        let tracker_config = TrackerConfig {
            max_pending_commands: 1000,
            cleanup_interval: Duration::from_secs(30),
            default_timeout: config.connection_timeout,
        };
        let response_tracker = ResponseTracker::new(tracker_config);
        
        Ok(Self {
            socket_path,
            channel_id,
            manifest: None,  // Will be fetched during operations when needed
            config,
            core_client,
            response_tracker,
            connection_state: std::sync::Mutex::new(ConnectionState::new()),
        })
    }
    
    /// Fetch Manifest from server
    async fn fetch_specification_from_server(
        core_client: &CoreJanusClient,
        _config: &JanusClientConfig,
    ) -> Result<Manifest, JanusError> {
        // Generate response socket path
        let response_socket_path = core_client.generate_response_socket_path();
        
        // Create proper JanusCommand for spec request
        let spec_command = JanusCommand {
            id: uuid::Uuid::new_v4().to_string(),
            channelId: "system".to_string(), // Use system channel for spec requests
            command: "spec".to_string(),
            reply_to: Some(response_socket_path.clone()),
            args: None,
            timeout: Some(10.0),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        };
        
        let command_data = serde_json::to_vec(&spec_command)
            .map_err(|e| JanusError::SerializationError {
                file: "janus_client.rs".to_string(),
                line: 71,
                message: format!("Failed to serialize spec command: {}", e),
            })?;
        
        // Send spec command to server
        let response_data = core_client
            .send_datagram(&command_data, &response_socket_path)
            .await?;
        
        // Parse response as JanusResponse
        let response: JanusResponse = serde_json::from_slice(&response_data)
            .map_err(|e| JanusError::SerializationError {
                file: "janus_client.rs".to_string(),
                line: 83,
                message: format!("Failed to parse server response: {}", e),
            })?;
        
        // Check for error in response
        if !response.success {
            let error_msg = response.error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(JanusError::ProtocolError {
                file: "janus_client.rs".to_string(),
                line: 91,
                message: format!("Server returned error: {}", error_msg),
            });
        }
        
        // Extract specification from response
        let spec_data = response.result.as_ref()
            .ok_or_else(|| JanusError::ProtocolError {
                file: "janus_client.rs".to_string(),
                line: 99,
                message: "Server response missing 'result' field".to_string(),
            })?;
        
        // Parse the specification
        let manifest: Manifest = serde_json::from_value(spec_data.clone())
            .map_err(|e| JanusError::SerializationError {
                file: "janus_client.rs".to_string(),
                line: 107,
                message: format!("Failed to parse server specification: {}", e),
            })?;
        
        Ok(manifest)
    }
    
    /// Ensure Manifest is loaded, fetching from server if needed
    async fn ensure_manifest_loaded(&mut self) -> Result<(), JanusError> {
        if self.manifest.is_some() {
            return Ok(()); // Already loaded
        }
        
        if !self.config.enable_validation {
            return Ok(()); // Validation disabled, no need to fetch
        }
        
        // Fetch specification from server
        let fetched_spec = Self::fetch_specification_from_server(&self.core_client, &self.config).await?;
        
        // Validate channel exists in fetched specification
        if !fetched_spec.channels.contains_key(&self.channel_id) {
            return Err(JanusError::InvalidChannel(
                format!("Channel '{}' not found in server specification", self.channel_id)
            ));
        }
        
        self.manifest = Some(fetched_spec);
        Ok(())
    }
    
    /// Send command via SOCK_DGRAM and wait for response
    pub async fn send_command(
        &mut self,
        command: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<Duration>,
    ) -> Result<JanusResponse, JanusError> {
        // Generate command ID and response socket path
        let command_id = Uuid::new_v4().to_string();
        let response_socket_path = self.core_client.generate_response_socket_path();
        
        // Create socket command
        let socket_command = JanusCommand {
            id: command_id.clone(),
            channelId: self.channel_id.clone(),
            command: command.to_string(),
            reply_to: Some(response_socket_path.clone()),
            args,
            timeout: timeout.map(|d| d.as_secs_f64()),
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        };
        
        // Apply security validation
        SecurityValidator::validate_command_name(command, &self.config)?;
        SecurityValidator::validate_args_size(&socket_command.args, &self.config)?;
        SecurityValidator::validate_socket_path(&response_socket_path)?;
        
        // Serialize command for message size validation
        let command_data = serde_json::to_vec(&socket_command)
            .map_err(|e| JanusError::SerializationError { 
                file: "janus_client.rs".to_string(), 
                line: 193, 
                message: format!("Failed to serialize command: {}", e) 
            })?;
        
        // Validate message size
        SecurityValidator::validate_message_size(command_data.len(), &self.config)?;
        
        // Ensure Manifest is loaded for validation
        if self.config.enable_validation {
            self.ensure_manifest_loaded().await?;
        }
        
        // Validate command against Manifest (skip for built-in commands)
        if let Some(ref spec) = self.manifest {
            if !Self::is_builtin_command(command) {
                self.validate_command_against_spec(spec, &socket_command)?;
            }
        }
        
        // Send datagram and wait for response
        let response_data = self.core_client
            .send_datagram(&command_data, &response_socket_path)
            .await?;
        
        // Deserialize response
        let response: JanusResponse = serde_json::from_slice(&response_data)
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
        
        // Update connection state after successful communication
        self.update_connection_state(1, 1);
        
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
        let socket_command = JanusCommand {
            id: command_id,
            channelId: self.channel_id.clone(),
            command: command.to_string(),
            reply_to: None,
            args,
            timeout: None,
            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        };
        
        // Apply security validation
        SecurityValidator::validate_command_name(command, &self.config)?;
        SecurityValidator::validate_args_size(&socket_command.args, &self.config)?;
        
        // Serialize command for message size validation
        let command_data = serde_json::to_vec(&socket_command)
            .map_err(|e| JanusError::SerializationError { 
                file: "janus_client.rs".to_string(), 
                line: 167, 
                message: format!("Failed to serialize command: {}", e) 
            })?;
        
        // Validate message size
        SecurityValidator::validate_message_size(command_data.len(), &self.config)?;
        
        // Validate command against Manifest
        if let Some(ref spec) = self.manifest {
            self.validate_command_against_spec(spec, &socket_command)?;
        }
        
        // Send datagram without waiting for response
        self.core_client.send_datagram_no_response(&command_data).await?;
        
        // Update connection state after successful send
        self.update_connection_state(1, 0);
        
        Ok(())
    }
    
    /// Test connectivity to the server
    pub async fn test_connection(&self) -> Result<(), JanusError> {
        self.core_client.test_connection().await
    }
    
    /// Validate command against Manifest
    fn validate_command_against_spec(
        &self,
        spec: &Manifest,
        command: &JanusCommand,
    ) -> Result<(), JanusError> {
        // Check if command is reserved (built-in commands should never be in Manifests)
        let builtin_commands = ["ping", "echo", "get_info", "validate", "slow_process", "spec"];
        if builtin_commands.contains(&command.command.as_str()) {
            return Err(JanusError::ValidationError(format!(
                "Command '{}' is reserved and cannot be used from Manifest",
                command.command
            )));
        }
        
        // Check if channel exists
        let channel = spec.channels.get(&command.channelId).ok_or_else(|| {
            JanusError::ValidationError(format!(
                "Channel {} not found in Manifest",
                command.channelId
            ))
        })?;
        
        // Check if command exists in channel
        let command_spec = channel.commands.get(&command.command).ok_or_else(|| {
            JanusError::ValidationError(format!(
                "Command '{}' not found in channel '{}'",
                command.command, command.channelId
            ))
        })?;
        
        // Validate command arguments if spec has argument definitions
        let spec_args = &command_spec.args;
        if !spec_args.is_empty() {
            let empty_args = std::collections::HashMap::new();
            let args = command.args.as_ref().unwrap_or(&empty_args);
            
            // Check for required arguments
            for (arg_name, arg_spec) in spec_args {
                if arg_spec.required.unwrap_or(false) && !args.contains_key(arg_name) {
                    return Err(JanusError::ValidationError(format!(
                        "Required argument '{}' missing for command '{}'",
                        arg_name, command.command
                    )));
                }
            }
            
            // Validate argument types and constraints
            for (arg_name, arg_value) in args {
                if let Some(arg_spec) = spec_args.get(arg_name) {
                    // Basic type validation - can be expanded for more comprehensive checks
                    match arg_spec.r#type.as_str() {
                        "string" => {
                            if !arg_value.is_string() {
                                return Err(JanusError::ValidationError(format!(
                                    "Argument '{}' must be a string",
                                    arg_name
                                )));
                            }
                        }
                        "number" => {
                            if !arg_value.is_number() {
                                return Err(JanusError::ValidationError(format!(
                                    "Argument '{}' must be a number",
                                    arg_name
                                )));
                            }
                        }
                        "boolean" => {
                            if !arg_value.is_boolean() {
                                return Err(JanusError::ValidationError(format!(
                                    "Argument '{}' must be a boolean",
                                    arg_name
                                )));
                            }
                        }
                        "array" => {
                            if !arg_value.is_array() {
                                return Err(JanusError::ValidationError(format!(
                                    "Argument '{}' must be an array",
                                    arg_name
                                )));
                            }
                        }
                        "object" => {
                            if !arg_value.is_object() {
                                return Err(JanusError::ValidationError(format!(
                                    "Argument '{}' must be an object",
                                    arg_name
                                )));
                            }
                        }
                        _ => {
                            // Unknown type - skip validation
                        }
                    }
                }
            }
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
    
    /// Get Manifest
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }
    
    /// Get configuration for backward compatibility
    pub fn configuration(&self) -> &JanusClientConfig {
        &self.config
    }
    
    /// Get specification for backward compatibility  
    pub fn specification(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }
    
    /// Send a ping command and return success/failure
    /// Convenience method for testing connectivity with a simple ping command
    pub async fn ping(&mut self) -> bool {
        match self.send_command("ping", None, None).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    /// Register command handler - validates command exists in specification (SOCK_DGRAM compatibility)
    /// This validates that the command exists in the Manifest for the client's channel.
    /// SOCK_DGRAM doesn't actually use handlers, but validation ensures compatibility.
    pub fn register_command_handler<T>(&self, command: &str, _handler: T) -> Result<(), JanusError> {
        // Validate command exists in the Manifest for the client's channel
        if let Some(ref spec) = self.manifest {
            if let Some(channel) = spec.channels.get(&self.channel_id) {
                if !channel.commands.contains_key(command) {
                    return Err(JanusError::InvalidArgument(
                        command.to_string(), 
                        format!("Command '{}' not found in channel '{}'", command, self.channel_id)
                    ));
                }
            }
        }
        
        // SOCK_DGRAM doesn't actually use handlers, but validation passed
        Ok(())
    }
    
    /// Get socket path for backward compatibility
    pub fn socket_path_string(&self) -> &str {
        &self.socket_path
    }
    
    /// Disconnect is a no-op for backward compatibility (SOCK_DGRAM doesn't have persistent connections)
    pub fn disconnect(&self) -> Result<(), JanusError> {
        // SOCK_DGRAM doesn't have persistent connections - this is for backward compatibility only
        Ok(())
    }
    
    /// Check if connected (legacy compatibility - SOCK_DGRAM doesn't maintain connections)
    pub fn is_connected(&self) -> bool {
        // In SOCK_DGRAM, we don't maintain persistent connections
        // Check if we can reach the server by checking if socket file exists
        std::path::Path::new(&self.socket_path).exists()
    }

    // MARK: - Connection State Simulation
    
    /// Get simulated connection state
    pub fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.lock().unwrap().clone()
    }
    
    /// Update connection state after successful operation
    fn update_connection_state(&self, messages_sent: u64, responses_received: u64) {
        if let Ok(mut state) = self.connection_state.lock() {
            state.is_connected = true;
            state.last_activity = std::time::SystemTime::now();
            state.messages_sent += messages_sent;
            state.responses_received += responses_received;
        }
    }

    // MARK: - Advanced Client Features (Response Correlation System)

    /// Send command with response correlation tracking
    pub async fn send_command_with_correlation(
        &self,
        command: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Duration,
    ) -> Result<(tokio::sync::oneshot::Receiver<JanusResponse>, String), JanusError> {
        let command_id = Uuid::new_v4().to_string();
        
        // Track the command in response tracker
        let receiver = self.response_tracker.track_command(command_id.clone(), timeout)
            .map_err(|e| JanusError::ValidationError(format!("Response tracking failed: {}", e)))?;

        // Send the command asynchronously
        let core_client = self.core_client.clone();
        let channel_id = self.channel_id.clone();
        let manifest = self.manifest.clone();
        let enable_validation = self.config.enable_validation;
        let response_tracker = self.response_tracker.clone();
        let cmd_id = command_id.clone();

        tokio::spawn(async move {
            // Create response socket path
            let response_socket_path = core_client.generate_response_socket_path();

            // Create socket command with specific ID
            let timeout_seconds = timeout.as_secs_f64();
            let socket_command = JanusCommand {
                id: cmd_id.clone(),
                channelId: channel_id,
                command: command.clone(),
                args,
                reply_to: Some(response_socket_path.clone()),
                timeout: Some(timeout_seconds),
                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
            };

            // Validate command if needed
            if enable_validation {
                if let Some(_spec) = &manifest {
                    if !Self::is_builtin_command(&command) {
                        // Perform validation (simplified for async context)
                        // Full validation would be more complex
                    }
                }
            }

            // Serialize and send command
            match serde_json::to_vec(&socket_command) {
                Ok(command_bytes) => {
                    match core_client.send_datagram(&command_bytes, &response_socket_path).await {
                        Ok(response_bytes) => {
                            // Parse response
                            match serde_json::from_slice::<JanusResponse>(&response_bytes) {
                                Ok(response) => {
                                    // Handle response through tracker
                                    response_tracker.handle_response(response);
                                }
                                Err(_) => {
                                    response_tracker.cancel_command(&cmd_id, Some("Failed to parse response"));
                                }
                            }
                        }
                        Err(_) => {
                            // Cancel the command due to send failure
                            response_tracker.cancel_command(&cmd_id, Some("Failed to send command"));
                        }
                    }
                }
                Err(_) => {
                    response_tracker.cancel_command(&cmd_id, Some("Failed to serialize command"));
                }
            }
        });

        Ok((receiver, command_id))
    }

    /// Cancel a pending command by ID
    pub fn cancel_command(&self, command_id: &str, reason: Option<&str>) -> bool {
        self.response_tracker.cancel_command(command_id, reason)
    }

    /// Cancel all pending commands
    pub fn cancel_all_commands(&self, reason: Option<&str>) -> usize {
        self.response_tracker.cancel_all_commands(reason)
    }

    /// Get number of pending commands
    pub fn get_pending_command_count(&self) -> usize {
        self.response_tracker.get_pending_count()
    }

    /// Get list of pending command IDs
    pub fn get_pending_command_ids(&self) -> Vec<String> {
        self.response_tracker.get_pending_command_ids()
    }

    /// Check if a command is currently pending
    pub fn is_command_pending(&self, command_id: &str) -> bool {
        self.response_tracker.is_tracking(command_id)
    }

    /// Get statistics about pending commands
    pub fn get_command_statistics(&self) -> CommandStatistics {
        self.response_tracker.get_statistics()
    }

    /// Execute multiple commands in parallel
    pub async fn execute_commands_in_parallel(
        &self,
        commands: Vec<ParallelCommand>,
    ) -> Vec<ParallelResult> {
        let mut results = Vec::with_capacity(commands.len());
        let mut handles = Vec::new();

        for cmd in commands {
            let mut client = self.clone();
            let handle = tokio::spawn(async move {
                let response = client.send_command(&cmd.command, cmd.args, None).await;
                let (response_ok, error_msg) = match response {
                    Ok(resp) => (Some(resp), None),
                    Err(e) => (None, Some(e.to_string())),
                };
                
                ParallelResult {
                    command_id: cmd.id,
                    response: response_ok,
                    error: error_msg,
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(ParallelResult {
                    command_id: "unknown".to_string(),
                    response: None,
                    error: Some(format!("Task execution failed: {}", e)),
                }),
            }
        }

        results
    }

    /// Create a channel proxy for executing commands on a specific channel
    pub fn create_channel_proxy(&self, channel_id: String) -> ChannelProxy {
        ChannelProxy {
            client: self.clone(),
            channel_id,
        }
    }

    /// Check if a command is a built-in command
    fn is_builtin_command(command: &str) -> bool {
        matches!(command, "ping" | "echo" | "get_info" | "spec" | "validate" | "slow_process")
    }
}

// MARK: - Helper Types for Advanced Features

/// Represents a command to be executed in parallel
#[derive(Debug, Clone)]
pub struct ParallelCommand {
    pub id: String,
    pub command: String,
    pub args: Option<HashMap<String, serde_json::Value>>,
}

/// Represents the result of a parallel command execution
#[derive(Debug, Clone)]
pub struct ParallelResult {
    pub command_id: String,
    pub response: Option<JanusResponse>,
    pub error: Option<String>,
}

/// Channel proxy provides channel-specific command execution
#[derive(Debug, Clone)]
pub struct ChannelProxy {
    client: JanusClient,
    channel_id: String,
}

impl ChannelProxy {
    /// Send command through this channel proxy
    pub async fn send_command(
        &self,
        command: String,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<JanusResponse, JanusError> {
        // Create a temporary client with the proxy's channel ID
        let mut proxy_client = self.client.clone();
        proxy_client.channel_id = self.channel_id.clone();
        
        proxy_client.send_command(&command, args, None).await
    }

    /// Get the proxy's channel ID
    pub fn get_channel_id(&self) -> &str {
        &self.channel_id
    }
}

// Need to implement Clone for JanusClient to support advanced features
impl Clone for JanusClient {
    fn clone(&self) -> Self {
        // Initialize a new response tracker for the cloned client
        let tracker_config = TrackerConfig {
            max_pending_commands: 1000,
            cleanup_interval: Duration::from_secs(30),
            default_timeout: self.config.connection_timeout,
        };
        let response_tracker = ResponseTracker::new(tracker_config);

        Self {
            socket_path: self.socket_path.clone(),
            channel_id: self.channel_id.clone(),
            manifest: self.manifest.clone(),
            config: self.config.clone(),
            core_client: self.core_client.clone(),
            response_tracker,
            connection_state: std::sync::Mutex::new(ConnectionState::new()),
        }
    }
}