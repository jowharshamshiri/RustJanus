use crate::specification::ApiSpecification;
use crate::config::UnixSockApiClientConfig;
use crate::error::{UnixSockApiError, Result};
use crate::protocol::{SocketCommand, SocketResponse, SocketMessage, MessageType, TimeoutHandler};
use crate::error::SocketError;
use crate::core::{SecurityValidator, ConnectionPool, UnixSocketClient};
use std::collections::HashMap;
use std::time::Duration;
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};
use tokio::time::timeout;
use uuid::Uuid;

/// Command handler type for async command processing
pub type CommandHandler = Arc<dyn Fn(SocketCommand, Option<HashMap<String, serde_json::Value>>) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Option<HashMap<String, serde_json::Value>>, UnixSockApiError>> + Send>> + Send + Sync>;

/// Response tracker for stateless commands
#[derive(Debug)]
struct ResponseTracker {
    command_id: String,
    channel_id: String,
    sender: oneshot::Sender<SocketResponse>,
    created_at: std::time::Instant,
    timeout: Duration,
}

impl ResponseTracker {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.timeout
    }
}

/// Main UnixSockApiClient (exact SwiftUnixSockAPI parity)
#[allow(non_snake_case)]
pub struct UnixSockApiClient {
    socket_path: String,
    channelId: String,
    api_spec: ApiSpecification,
    config: UnixSockApiClientConfig,
    connection_pool: Arc<ConnectionPool>,
    command_handlers: Arc<Mutex<HashMap<String, CommandHandler>>>,
    pending_commands: Arc<Mutex<HashMap<String, oneshot::Sender<SocketResponse>>>>,
    response_trackers: Arc<Mutex<HashMap<String, ResponseTracker>>>,
    persistent_connection: Arc<Mutex<Option<UnixSocketClient>>>,
}

impl UnixSockApiClient {
    /// Create a new UnixSockApiClient
    #[allow(non_snake_case)]
    pub async fn new(
        socket_path: String,
        channelId: String,
        api_spec: ApiSpecification,
        config: UnixSockApiClientConfig,
    ) -> Result<Self> {
        // Validate socket path
        SecurityValidator::validate_socket_path(&socket_path)?;
        
        // Validate channel ID
        SecurityValidator::validate_channel_id(&channelId, &config)?;
        
        // Validate configuration
        config.validate()
            .map_err(|e| UnixSockApiError::ValidationError(e))?;
            
        // Validate that the channel exists in the API spec
        if !api_spec.has_channel(&channelId) {
            return Err(UnixSockApiError::InvalidChannel(channelId));
        }
        
        let connection_pool = Arc::new(ConnectionPool::new(socket_path.clone(), config.clone()));
        
        Ok(Self {
            socket_path,
            channelId,
            api_spec,
            config,
            connection_pool,
            command_handlers: Arc::new(Mutex::new(HashMap::new())),
            pending_commands: Arc::new(Mutex::new(HashMap::new())),
            response_trackers: Arc::new(Mutex::new(HashMap::new())),
            persistent_connection: Arc::new(Mutex::new(None)),
        })
    }
    
    /// Send a command with timeout
    pub async fn send_command(
        &self,
        command_name: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout_duration: Duration,
        on_timeout: Option<TimeoutHandler>,
    ) -> Result<SocketResponse> {
        // Validate command name
        SecurityValidator::validate_command_name(command_name, &self.config)?;
        
        // Validate arguments size
        SecurityValidator::validate_args_size(&args, &self.config)?;
        
        // Validate command against API spec
        self.validate_command(command_name, &args)?;
        
        // Check pending command limit
        {
            let pending = self.pending_commands.lock().await;
            if pending.len() >= self.config.max_pending_commands {
                return Err(UnixSockApiError::ResourceLimit(
                    "Maximum pending commands exceeded".to_string()
                ));
            }
        }
        
        let command_id = Uuid::new_v4().to_string();
        let command = SocketCommand::new(
            self.channelId.clone(),
            command_name.to_string(),
            args,
            Some(timeout_duration.as_secs_f64()),
        );
        
        // Get connection from pool
        let socket_client = self.connection_pool.borrow_connection().await?;
        
        // Create response channel
        let (tx, rx) = oneshot::channel();
        
        // Register pending command
        {
            let mut pending = self.pending_commands.lock().await;
            pending.insert(command_id.clone(), tx);
        }
        
        // Create message
        let message = SocketMessage {
            message_type: MessageType::Command,
            payload: serde_json::to_vec(&command)
                .map_err(|e| UnixSockApiError::EncodingFailed(e.to_string()))?,
        };
        
        let message_data = serde_json::to_vec(&message)
            .map_err(|e| UnixSockApiError::EncodingFailed(e.to_string()))?;
        
        // Send command and wait for response with timeout
        let result = timeout(timeout_duration, async {
            // Send message
            socket_client.send_message(&message_data).await?;
            
            // Wait for response
            rx.await.map_err(|_| UnixSockApiError::CommandTimeout(
                command_id.clone(),
                timeout_duration
            ))
        }).await;
        
        // Return connection to pool
        self.connection_pool.return_connection(socket_client).await;
        
        // Clean up pending command
        {
            let mut pending = self.pending_commands.lock().await;
            pending.remove(&command_id);
        }
        
        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                if let Some(handler) = on_timeout {
                    handler(command_id.clone(), timeout_duration);
                }
                Err(UnixSockApiError::CommandTimeout(command_id, timeout_duration))
            }
        }
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
        
        // Validate command against API spec
        self.validate_command(command_name, &args)?;
        
        let command_id = Uuid::new_v4().to_string();
        let command = SocketCommand::new(
            self.channelId.clone(),
            command_name.to_string(),
            args,
            None,
        );
        
        // Create ephemeral connection for fire-and-forget
        let socket_client = UnixSocketClient::new(
            self.socket_path.clone(),
            self.config.clone()
        )?;
        
        // Create message
        let message = SocketMessage {
            message_type: MessageType::Command,
            payload: serde_json::to_vec(&command)
                .map_err(|e| UnixSockApiError::EncodingFailed(e.to_string()))?,
        };
        
        let message_data = serde_json::to_vec(&message)
            .map_err(|e| UnixSockApiError::EncodingFailed(e.to_string()))?;
        
        // Send without waiting for response
        socket_client.send_message_no_response(&message_data).await?;
        
        Ok(command_id)
    }
    
    /// Register a command handler
    pub async fn register_command_handler(
        &self,
        command_name: &str,
        handler: CommandHandler,
    ) -> Result<()> {
        // Validate command name
        SecurityValidator::validate_command_name(command_name, &self.config)?;
        
        // Check handler limit
        {
            let handlers = self.command_handlers.lock().await;
            if handlers.len() >= self.config.max_command_handlers {
                return Err(UnixSockApiError::ResourceLimit(
                    "Maximum command handlers exceeded".to_string()
                ));
            }
        }
        
        // Validate that the command exists in the API spec
        if !self.api_spec.has_command(&self.channelId, command_name) {
            return Err(UnixSockApiError::UnknownCommand(command_name.to_string()));
        }
        
        // Register handler
        {
            let mut handlers = self.command_handlers.lock().await;
            handlers.insert(command_name.to_string(), handler);
        }
        
        Ok(())
    }
    
    /// Start listening for commands
    /// Logic: if handlers registered -> create server socket, if no handlers -> client mode
    pub async fn start_listening(&self) -> Result<()> {
        // Check if we have handlers registered (expecting requests mode)
        let has_handlers = {
            let handlers = self.command_handlers.lock().await;
            !handlers.is_empty()
        };
        
        if has_handlers {
            // Server mode: create Unix domain socket server and listen for connections
            self.start_server_mode().await
        } else {
            // Client mode: connect to existing socket for receiving responses
            self.start_client_mode().await
        }
    }
    
    /// Server mode: create Unix domain socket server and listen for connections
    async fn start_server_mode(&self) -> Result<()> {
        use tokio::net::UnixListener;
        
        // Remove existing socket file if it exists
        let _ = std::fs::remove_file(&self.socket_path);
        
        // Create Unix domain socket listener
        let listener = UnixListener::bind(&self.socket_path)
            .map_err(|e| UnixSockApiError::ConnectionError(format!("Failed to bind server socket: {}", e)))?;
        
        // Accept connections in background task
        let channel_id = self.channelId.clone();
        let handlers = self.command_handlers.clone();
        
        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, _)) => {
                        let channel_id_clone = channel_id.clone();
                        let handlers_clone = handlers.clone();
                        tokio::spawn(async move {
                            Self::handle_server_connection(stream, channel_id_clone, handlers_clone).await;
                        });
                    },
                    Err(_) => break,
                }
            }
        });
        
        Ok(())
    }
    
    /// Client mode: connect to existing socket for receiving responses
    async fn start_client_mode(&self) -> Result<()> {
        let socket_client = UnixSocketClient::new(
            self.socket_path.clone(),
            self.config.clone()
        )?;
        
        // Store persistent connection
        {
            let mut persistent = self.persistent_connection.lock().await;
            *persistent = Some(socket_client);
        }
        
        // Start message handling loop
        self.start_message_handling_loop().await?;
        
        Ok(())
    }
    
    /// Handle incoming connections in server mode
    async fn handle_server_connection(
        mut stream: tokio::net::UnixStream,
        channel_id: String,
        handlers: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<String, CommandHandler>>>,
    ) {
        // Removed unused AsyncReadExt and AsyncWriteExt imports
        use crate::core::message_framing::MessageFrame;
        
        loop {
            // Read framed message
            let data = match MessageFrame::read_frame(&mut stream).await {
                Ok(data) => data,
                Err(_) => break, // Connection closed or error
            };
            
            // Try to parse as SocketCommand directly (Go sends raw commands)
            match serde_json::from_slice::<SocketCommand>(&data) {
                Ok(command) => {
                    // Only process commands for our channel
                    if command.channelId == channel_id {
                        // Get handler
                        let handler = {
                                    let handlers_lock = handlers.lock().await;
                                    handlers_lock.get(&command.command).cloned()
                        };
                        
                        if let Some(handler) = handler {
                            // Execute handler
                            let args = command.args.clone();
                            let result = handler(command.clone(), args).await;
                            
                            // Create response
                            let response = match result {
                                Ok(Some(result)) => SocketResponse {
                                    commandId: command.id.clone(),
                                    channelId: command.channelId.clone(),
                                    success: true,
                                    result: Some(serde_json::Value::Object(result.into_iter().collect())),
                                    error: None,
                                    timestamp: chrono::Utc::now(),
                                },
                                Ok(None) => SocketResponse {
                                    commandId: command.id.clone(),
                                    channelId: command.channelId.clone(),
                                    success: true,
                                    result: None,
                                    error: None,
                                    timestamp: chrono::Utc::now(),
                                },
                                Err(e) => SocketResponse {
                                    commandId: command.id.clone(),
                                    channelId: command.channelId.clone(),
                                    success: false,
                                    result: None,
                                    error: Some(SocketError::ProcessingError(e.to_string())),
                                    timestamp: chrono::Utc::now(),
                                },
                            };
                            
                            // Send response directly (Go expects raw response, not wrapped in SocketMessage)
                            let response_data = serde_json::to_vec(&response).unwrap_or_default();
                            
                            // Write framed response
                            let _ = MessageFrame::write_frame(&mut stream, &response_data).await;
                        }
                    }
                },
                Err(_) => {
                    // Invalid message format, ignore
                }
            }
        }
    }
    
    /// Get configuration
    pub fn configuration(&self) -> &UnixSockApiClientConfig {
        &self.config
    }
    
    /// Get specification
    pub fn specification(&self) -> &ApiSpecification {
        &self.api_spec
    }
    
    /// Validate command against API specification
    fn validate_command(
        &self,
        command_name: &str,
        _args: &Option<HashMap<String, serde_json::Value>>
    ) -> Result<()> {
        if !self.api_spec.has_command(&self.channelId, command_name) {
            return Err(UnixSockApiError::UnknownCommand(command_name.to_string()));
        }
        
        // Additional validation could be added here for required arguments
        // based on the API specification
        
        Ok(())
    }
    
    /// Start the message handling loop for persistent listening
    async fn start_message_handling_loop(&self) -> Result<()> {
        // This would typically run in a background task
        // For now, we'll just set up the infrastructure
        
        // Clean up expired trackers periodically
        self.cleanup_expired_trackers().await;
        
        Ok(())
    }
    
    /// Clean up expired response trackers
    async fn cleanup_expired_trackers(&self) {
        let mut trackers = self.response_trackers.lock().await;
        let expired_ids: Vec<String> = trackers
            .iter()
            .filter(|(_, tracker)| tracker.is_expired())
            .map(|(id, _)| id.clone())
            .collect();
            
        for id in expired_ids {
            trackers.remove(&id);
        }
    }
    
    /// Handle incoming messages from the socket
    async fn handle_incoming_message(&self, data: &[u8]) -> Result<()> {
        let message: SocketMessage = serde_json::from_slice(data)
            .map_err(|e| UnixSockApiError::DecodingFailed(e.to_string()))?;
            
        match message.message_type {
            MessageType::Command => self.handle_incoming_command(&message.payload).await,
            MessageType::Response => self.handle_incoming_response(&message.payload).await,
        }
    }
    
    /// Handle incoming command messages
    async fn handle_incoming_command(&self, payload: &[u8]) -> Result<()> {
        let command: SocketCommand = serde_json::from_slice(payload)
            .map_err(|e| UnixSockApiError::DecodingFailed(e.to_string()))?;
            
        // Only process commands for our channel
        if command.channelId != self.channelId {
            return Ok(());
        }
        
        // Check if we have a handler for this command
        let handler = {
            let handlers = self.command_handlers.lock().await;
            handlers.get(&command.command).cloned()
        };
        
        if let Some(handler) = handler {
            // Execute command handler
            let result = if let Some(timeout_secs) = command.timeout {
                let timeout_duration = Duration::from_secs_f64(timeout_secs);
                timeout(timeout_duration, handler(command.clone(), command.args.clone())).await
                    .map_err(|_| UnixSockApiError::HandlerTimeout(
                        command.id.clone(),
                        timeout_duration
                    ))?
            } else {
                handler(command.clone(), command.args.clone()).await
            };
            
            match result {
                Ok(_response_data) => {
                    println!("Command '{}' executed successfully", command.command);
                    // Response would be sent back via the socket
                }
                Err(e) => {
                    println!("Command '{}' failed: {:?}", command.command, e);
                    // Error response would be sent back via the socket
                }
            }
        } else {
            println!("Unknown command '{}' received on channel '{}'", command.command, self.channelId);
        }
        
        Ok(())
    }
    
    /// Handle incoming response messages
    async fn handle_incoming_response(&self, payload: &[u8]) -> Result<()> {
        let response: SocketResponse = serde_json::from_slice(payload)
            .map_err(|e| UnixSockApiError::DecodingFailed(e.to_string()))?;
            
        // Only process responses for our channel
        if response.channelId != self.channelId {
            return Ok(());
        }
        
        // Route response to the correct pending command
        let sender = {
            let mut pending = self.pending_commands.lock().await;
            pending.remove(&response.commandId)
        };
        
        if let Some(sender) = sender {
            let _ = sender.send(response);
        }
        
        Ok(())
    }
}