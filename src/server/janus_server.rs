use std::collections::HashMap;
use std::os::unix::net::Janus;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use serde_json;
use std::fs;

use crate::protocol::message_types::{SocketCommand, SocketResponse};
use crate::error::JanusError;

/// Command handler function type for SOCK_DGRAM server
pub type JanusCommandHandler = Box<dyn Fn(SocketCommand) -> Result<serde_json::Value, JanusError> + Send + Sync>;

/// High-level SOCK_DGRAM Unix socket server
/// Handles command routing and response generation for connectionless communication
pub struct JanusServer {
    handlers: Arc<Mutex<HashMap<String, JanusCommandHandler>>>,
    is_running: Arc<AtomicBool>,
    socket_path: Option<String>,
}

impl JanusServer {
    /// Create a new SOCK_DGRAM server
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
            is_running: Arc::new(AtomicBool::new(false)),
            socket_path: None,
        }
    }

    /// Register a command handler
    pub async fn register_handler<F>(&mut self, command: &str, handler: F)
    where
        F: Fn(SocketCommand) -> Result<serde_json::Value, JanusError> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.lock().await;
        handlers.insert(command.to_string(), Box::new(handler));
    }

    /// Start listening on the specified socket path using SOCK_DGRAM
    /// Returns immediately, runs server in background task
    pub async fn start_listening(&mut self, socket_path: &str) -> Result<(), JanusError> {
        self.socket_path = Some(socket_path.to_string());
        self.is_running.store(true, Ordering::SeqCst);

        let path = socket_path.to_string();
        let handlers = Arc::clone(&self.handlers);
        let is_running = Arc::clone(&self.is_running);

        tokio::spawn(async move {
            if let Err(e) = Self::listen_loop(path, handlers, is_running).await {
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
        
        // Clean up socket file
        if let Some(ref path) = self.socket_path {
            let _ = fs::remove_file(path);
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
        is_running: Arc<AtomicBool>,
    ) -> Result<(), JanusError> {
        // Remove existing socket
        let _ = fs::remove_file(&socket_path);

        let socket = Janus::bind(&socket_path)
            .map_err(|e| JanusError::IoError(format!("Failed to bind socket: {}", e)))?;

        // Set non-blocking mode for graceful shutdown
        socket.set_nonblocking(true)
            .map_err(|e| JanusError::IoError(format!("Failed to set non-blocking: {}", e)))?;

        println!("SOCK_DGRAM server listening on: {}", socket_path);

        while is_running.load(Ordering::SeqCst) {
            let mut buffer = vec![0u8; 64 * 1024];
            
            match socket.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    let data = &buffer[..size];
                    Self::process_datagram(data, &handlers).await;
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
    ) {
        match serde_json::from_slice::<SocketCommand>(data) {
            Ok(cmd) => {
                println!("Received SOCK_DGRAM command: {} (ID: {})", cmd.command, cmd.id);

                // Process command and send response if reply_to is specified
                if let Some(ref reply_to) = cmd.reply_to {
                    let response = Self::process_command(&cmd, handlers).await;
                    Self::send_response(response, reply_to).await;
                }
            }
            Err(e) => {
                eprintln!("Failed to parse datagram: {}", e);
            }
        }
    }

    async fn process_command(
        cmd: &SocketCommand,
        handlers: &Arc<Mutex<HashMap<String, JanusCommandHandler>>>,
    ) -> SocketResponse {
        let handlers_guard = handlers.lock().await;
        
        let response = if let Some(handler) = handlers_guard.get(&cmd.command) {
            match handler(cmd.clone()) {
                Ok(result) => SocketResponse {
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
                Err(e) => SocketResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: false,
                    result: None,
                    error: Some(crate::error::SocketError::ProcessingError(e.to_string())),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
            }
        } else {
            // Default handlers (matching main binary)
            match cmd.command.as_str() {
                "ping" => SocketResponse {
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
                    
                    SocketResponse {
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
                _ => SocketResponse {
                    commandId: cmd.id.clone(),
                    channelId: cmd.channelId.clone(),
                    success: false,
                    result: None,
                    error: Some(crate::error::SocketError::ProcessingError(
                        format!("Command '{}' not registered", cmd.command)
                    )),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64(),
                },
            }
        };

        response
    }

    async fn send_response(response: SocketResponse, reply_to: &str) {
        match serde_json::to_vec(&response) {
            Ok(response_data) => {
                if let Ok(client_sock) = Janus::unbound() {
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
        Self::new()
    }
}