use std::collections::HashMap;
use std::os::unix::net::UnixDatagram;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use serde_json;
use std::fs;

use crate::protocol::message_types::{JanusCommand, JanusResponse};
use crate::error::{JSONRPCError, JSONRPCErrorCode};

/// Server configuration structure matching other implementations
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub socket_path: String,
    pub max_connections: usize,
    pub default_timeout: u64,
    pub max_message_size: usize,
    pub cleanup_on_start: bool,
    pub cleanup_on_shutdown: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            socket_path: String::new(),
            max_connections: 100,
            default_timeout: 30,
            max_message_size: 65536,
            cleanup_on_start: true,
            cleanup_on_shutdown: true,
        }
    }
}

/// Command handler function type for SOCK_DGRAM server (sync)
pub type JanusCommandHandler = Box<dyn Fn(JanusCommand) -> Result<serde_json::Value, JSONRPCError> + Send + Sync>;

/// Async command handler function type for SOCK_DGRAM server
pub type JanusAsyncCommandHandler = Box<dyn Fn(JanusCommand) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, JSONRPCError>> + Send>> + Send + Sync>;

/// High-level SOCK_DGRAM Unix socket server
/// Handles command routing and response generation for connectionless communication
pub struct JanusServer {
    config: ServerConfig,
    handlers: Arc<Mutex<HashMap<String, JanusCommandHandler>>>,
    async_handlers: Arc<Mutex<HashMap<String, JanusAsyncCommandHandler>>>,
    is_running: Arc<AtomicBool>,
}

impl JanusServer {
    /// Create a new SOCK_DGRAM server with configuration
    /// Matches constructor signatures of Go, Swift, and TypeScript implementations
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(Mutex::new(HashMap::new())),
            async_handlers: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register a command handler (synchronous)
    pub async fn register_handler<F>(&mut self, command: &str, handler: F)
    where
        F: Fn(JanusCommand) -> Result<serde_json::Value, JSONRPCError> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.lock().await;
        handlers.insert(command.to_string(), Box::new(handler));
    }

    /// Register an asynchronous command handler
    pub async fn register_async_handler<F, Fut>(&mut self, command: &str, handler: F)
    where
        F: Fn(JanusCommand) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, JSONRPCError>> + Send + 'static,
    {
        let async_handler: JanusAsyncCommandHandler = Box::new(move |cmd| {
            Box::pin(handler(cmd))
        });
        let mut async_handlers = self.async_handlers.lock().await;
        async_handlers.insert(command.to_string(), async_handler);
    }

    /// Start listening on the configured socket path using SOCK_DGRAM
    /// Returns immediately, runs server in background task
    pub async fn start_listening(&mut self) -> Result<(), JSONRPCError> {
        if self.config.socket_path.is_empty() {
            return Err(JSONRPCError::new(
                JSONRPCErrorCode::InvalidRequest,
                Some("Socket path not configured".to_string()),
            ));
        }
        
        // Clean up existing socket file if configured
        if self.config.cleanup_on_start {
            let _ = fs::remove_file(&self.config.socket_path);
        }
        
        self.is_running.store(true, Ordering::SeqCst);

        let path = self.config.socket_path.clone();
        let handlers = Arc::clone(&self.handlers);
        let async_handlers = Arc::clone(&self.async_handlers);
        let is_running = Arc::clone(&self.is_running);
        let cleanup_on_shutdown = self.config.cleanup_on_shutdown;

        tokio::spawn(async move {
            if let Err(e) = Self::listen_loop(path, handlers, async_handlers, is_running).await {
                eprintln!("Server error: {}", e);
            }
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        Ok(())
    }

    /// Stop the server
    pub fn stop(&self) {
        self.is_running.store(false, Ordering::SeqCst);
        
        // Clean up socket file if configured
        if self.config.cleanup_on_shutdown && !self.config.socket_path.is_empty() {
            let _ = fs::remove_file(&self.config.socket_path);
        }
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    // Private implementation
    async fn listen_loop(
        socket_path: String,
        handlers: Arc<Mutex<HashMap<String, JanusCommandHandler>>>,
        async_handlers: Arc<Mutex<HashMap<String, JanusAsyncCommandHandler>>>,
        is_running: Arc<AtomicBool>,
    ) -> Result<(), JSONRPCError> {
        // Remove existing socket
        let _ = fs::remove_file(&socket_path);

        let socket = UnixDatagram::bind(&socket_path)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to bind socket: {}", e))))?;

        // Set non-blocking mode for graceful shutdown
        socket.set_nonblocking(true)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to set non-blocking: {}", e))))?;

        println!("SOCK_DGRAM server listening on: {}", socket_path);

        while is_running.load(Ordering::SeqCst) {
            let mut buffer = vec![0u8; 64 * 1024];
            
            match socket.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    let data = &buffer[..size];
                    Self::process_datagram(data, &handlers, &async_handlers).await;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Non-blocking socket would block, continue polling
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => {
                    eprintln!("Error receiving datagram: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        // Cleanup
        let _ = fs::remove_file(&socket_path);
        println!("SOCK_DGRAM server stopped");
        Ok(())
    }

    async fn process_datagram(
        data: &[u8],
        handlers: &Arc<Mutex<HashMap<String, JanusCommandHandler>>>,
        async_handlers: &Arc<Mutex<HashMap<String, JanusAsyncCommandHandler>>>,
    ) {
        match serde_json::from_slice::<JanusCommand>(data) {
            Ok(cmd) => {
                println!("Received SOCK_DGRAM command: {} (ID: {})", cmd.command, cmd.id);

                // Process command and send response if reply_to is specified
                if let Some(ref reply_to) = cmd.reply_to {
                    let response = Self::process_command(&cmd, handlers, async_handlers).await;
                    Self::send_response(response, reply_to).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to parse datagram: {}", e);
            }
        }
    }

    async fn process_command(
        cmd: &JanusCommand,
        handlers: &Arc<Mutex<HashMap<String, JanusCommandHandler>>>,
        async_handlers: &Arc<Mutex<HashMap<String, JanusAsyncCommandHandler>>>,
    ) -> JanusResponse {
        // Check async handlers first
        let async_handlers_guard = async_handlers.lock().await;
        let response = if let Some(async_handler) = async_handlers_guard.get(&cmd.command) {
            let future = async_handler(cmd.clone());
            drop(async_handlers_guard); // Release lock before await
            match future.await {
                Ok(result) => JanusResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: true,
                    result: Some(result),
                    error: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
                Err(e) => JanusResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: false,
                    result: None,
                    error: Some(e),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
            }
        } else {
            drop(async_handlers_guard);
            
            // Check sync handlers
            let handlers_guard = handlers.lock().await;
            if let Some(handler) = handlers_guard.get(&cmd.command) {
                match handler(cmd.clone()) {
                    Ok(result) => JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    },
                    Err(e) => JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: false,
                        result: None,
                        error: Some(e),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    },
                }
            } else {
                drop(handlers_guard);
                
                // Default handlers (matching main binary)
                match cmd.command.as_str() {
                "ping" => JanusResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: true,
                    result: Some(serde_json::json!({
                        "pong": true,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64()
                    })),
                    error: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
                "echo" => {
                    let message = cmd.args.as_ref()
                        .and_then(|args| args.get("message"))
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::String("Hello from Rust SOCK_DGRAM server!".to_string()));
                    
                    JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: true,
                        result: Some(serde_json::json!({"echo": message})),
                        error: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    }
                }
                "get_info" => JanusResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: true,
                    result: Some(serde_json::json!({
                        "implementation": "Rust",
                        "version": "1.0.0",
                        "protocol": "SOCK_DGRAM",
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64()
                    })),
                    error: None,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
                "validate" => {
                    let result = if let Some(message) = cmd.args.as_ref()
                        .and_then(|args| args.get("message"))
                        .and_then(|v| v.as_str()) {
                        match serde_json::from_str::<serde_json::Value>(message) {
                            Ok(parsed) => serde_json::json!({
                                "valid": true,
                                "data": parsed
                            }),
                            Err(e) => serde_json::json!({
                                "valid": false,
                                "error": "Invalid JSON format",
                                "reason": e.to_string()
                            })
                        }
                    } else {
                        serde_json::json!({
                            "valid": false,
                            "error": "No message provided for validation"
                        })
                    };
                    
                    JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    }
                }
                "slow_process" => {
                    // Simulate 2-second delay like other implementations
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    
                    let mut result = serde_json::json!({
                        "processed": true,
                        "delay": "2000ms"
                    });
                    
                    // Include message if provided
                    if let Some(message) = cmd.args.as_ref()
                        .and_then(|args| args.get("message")) {
                        result["message"] = message.clone();
                    }
                    
                    JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: true,
                        result: Some(result),
                        error: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    }
                }
                "spec" => {
                    // Return a proper Manifest with test channels
                    let spec = serde_json::json!({
                        "version": "1.0.0",
                        "channels": {
                            "test": {
                                "description": "Test channel for cross-platform communication",
                                "commands": {
                                    "test_echo": {
                                        "description": "Echo test command",
                                        "args": {
                                            "message": {
                                                "type": "string",
                                                "required": false,
                                                "description": "Message to echo back"
                                            }
                                        },
                                        "response": {
                                            "type": "object",
                                            "properties": {
                                                "echo": {
                                                    "type": "string"
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "test_channel": {
                                "description": "Test channel for high-level API tests",
                                "commands": {
                                    "test_echo": {
                                        "description": "Test echo command for high-level API tests",
                                        "args": {
                                            "message": {
                                                "type": "string",
                                                "required": false,
                                                "description": "Message to echo back"
                                            }
                                        },
                                        "response": {
                                            "type": "object",
                                            "properties": {
                                                "echo": {
                                                    "type": "string"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "models": {}
                    });
                    
                    JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: true,
                        result: Some(spec),
                        error: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    }
                }
                "test_echo" => {
                    // Handle test_echo command for high-level API tests
                    let message = cmd.args.as_ref()
                        .and_then(|args| args.get("message"))
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::String("Hello Server!".to_string()));
                    
                    JanusResponse {
                        commandId: cmd.id.clone(),
                        channelId: cmd.channelId.clone(),
                        success: true,
                        result: Some(serde_json::json!({"echo": message})),
                        error: None,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64(),
                    }
                }
                _ => JanusResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: false,
                    result: None,
                    error: Some({
                        use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode};
                        JSONRPCError::new(
                            JSONRPCErrorCode::MethodNotFound,
                            Some(format!("Command '{}' not registered", cmd.command))
                        )
                    }),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
                }
            }
        };

        response
    }

    async fn send_response(response: JanusResponse, reply_to: &str) {
        match serde_json::to_vec(&response) {
            Ok(response_data) => {
                if let Ok(client_sock) = UnixDatagram::unbound() {
                    if let Err(e) = client_sock.send_to(&response_data, reply_to) {
                        eprintln!("Error sending response: {}", e);
                    } else {
                        println!("Response sent to: {}", reply_to);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error serializing response: {}", e);
            }
        }
    }
}

impl Drop for JanusServer {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Default for JanusServer {
    fn default() -> Self {
        Self::new(ServerConfig::default())
    }
}