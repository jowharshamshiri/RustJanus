use std::collections::HashMap;
use std::os::unix::net::UnixDatagram;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use serde_json;
use std::fs;

use crate::protocol::message_types::{JanusRequest, JanusResponse};
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

/// Request handler function type for SOCK_DGRAM server (sync)
pub type JanusRequestHandler = Box<dyn Fn(JanusRequest) -> Result<serde_json::Value, JSONRPCError> + Send + Sync>;

/// Async request handler function type for SOCK_DGRAM server
pub type JanusAsyncRequestHandler = Box<dyn Fn(JanusRequest) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, JSONRPCError>> + Send>> + Send + Sync>;

/// High-level SOCK_DGRAM Unix socket server
/// Handles request routing and response generation for connectionless communication
pub struct JanusServer {
    config: ServerConfig,
    handlers: Arc<Mutex<HashMap<String, JanusRequestHandler>>>,
    async_handlers: Arc<Mutex<HashMap<String, JanusAsyncRequestHandler>>>,
    is_running: Arc<AtomicBool>,
    server_task: Option<JoinHandle<Result<(), JSONRPCError>>>,
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
            server_task: None,
        }
    }

    /// Register a request handler (synchronous)
    pub async fn register_handler<F>(&mut self, request: &str, handler: F)
    where
        F: Fn(JanusRequest) -> Result<serde_json::Value, JSONRPCError> + Send + Sync + 'static,
    {
        let mut handlers = self.handlers.lock().await;
        handlers.insert(request.to_string(), Box::new(handler));
    }

    /// Register an asynchronous request handler
    pub async fn register_async_handler<F, Fut>(&mut self, request: &str, handler: F)
    where
        F: Fn(JanusRequest) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<serde_json::Value, JSONRPCError>> + Send + 'static,
    {
        let async_handler: JanusAsyncRequestHandler = Box::new(move |cmd| {
            Box::pin(handler(cmd))
        });
        let mut async_handlers = self.async_handlers.lock().await;
        async_handlers.insert(request.to_string(), async_handler);
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
        let _cleanup_on_shutdown = self.config.cleanup_on_shutdown;

        // Spawn the listen loop and store the task handle
        let task_handle = tokio::spawn(Self::listen_loop(path, handlers, async_handlers, is_running));
        self.server_task = Some(task_handle);
        
        // Give the server a moment to bind the socket
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(())
    }

    /// Stop the server
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        
        // Clean up socket file if configured
        if self.config.cleanup_on_shutdown && !self.config.socket_path.is_empty() {
            let _ = fs::remove_file(&self.config.socket_path);
        }
        
        // Wait for server task to complete if it exists
        if let Some(task) = self.server_task.take() {
            task.abort(); // Abort the task since we're stopping
        }
    }

    /// Check if server is running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    /// Wait for the server to complete (blocks until server stops)
    pub async fn wait_for_completion(&mut self) -> Result<(), crate::error::JSONRPCError> {
        if let Some(task_handle) = self.server_task.take() {
            match task_handle.await {
                Ok(result) => result,
                Err(e) => {
                    Err(crate::error::JSONRPCError::new(
                        crate::error::JSONRPCErrorCode::InternalError,
                        Some(format!("Server task failed: {}", e))
                    ))
                }
            }
        } else {
            Err(crate::error::JSONRPCError::new(
                crate::error::JSONRPCErrorCode::InternalError,
                Some("No server task running".to_string())
            ))
        }
    }

    // Private implementation (public for testing)
    pub async fn listen_loop(
        socket_path: String,
        _handlers: Arc<Mutex<HashMap<String, JanusRequestHandler>>>,
        _async_handlers: Arc<Mutex<HashMap<String, JanusAsyncRequestHandler>>>,
        is_running: Arc<AtomicBool>,
    ) -> Result<(), JSONRPCError> {
        println!("DEBUG: listen_loop starting for socket: {}", socket_path);
        
        // Remove existing socket
        let _ = fs::remove_file(&socket_path);

        let socket = UnixDatagram::bind(&socket_path)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to bind socket: {}", e))))?;
        
        // Set non-blocking mode for async-like behavior
        socket.set_nonblocking(true)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::SocketError, Some(format!("Failed to set non-blocking: {}", e))))?;
        
        println!("DEBUG: Socket bound successfully in non-blocking mode");

        println!("SOCK_DGRAM server listening on: {}", socket_path);

        let mut poll_count = 0;
        let mut data_received = false;
        println!("DEBUG: Server loop starting, is_running={}", is_running.load(Ordering::SeqCst));
        
        while is_running.load(Ordering::SeqCst) {
            poll_count += 1;
            if poll_count == 1 {
                println!("DEBUG: First loop iteration, is_running={}", is_running.load(Ordering::SeqCst));
            }
            if poll_count % 50 == 0 {
                println!("DEBUG: Server poll count: {}, still running, data_received={}", poll_count, data_received);
            }
            let mut buffer = vec![0u8; 64 * 1024];
            
            if poll_count <= 5 || poll_count % 100 == 0 {
                println!("DEBUG: About to call recv_from, poll_count={}", poll_count);
            }
            
            // Try to receive in non-blocking mode
            match socket.recv_from(&mut buffer) {
                Ok((size, sender_addr)) => {
                    data_received = true;
                    let receive_time = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs_f64();
                    let data = &buffer[..size];
                    let sender_path = sender_addr.as_pathname()
                        .and_then(|p| p.to_str())
                        .unwrap_or("<unknown>")
                        .to_string();
                    println!("DEBUG: Received datagram of {} bytes from {} at timestamp {:.3}", size, sender_path, receive_time);
                    
                    // Process with custom handlers support
                    match serde_json::from_slice::<JanusRequest>(data) {
                        Ok(cmd) => {
                            println!("DEBUG: Received SOCK_DGRAM request: {} (ID: {})", cmd.request, cmd.id);
                            println!("DEBUG: Request reply_to field: {:?}", cmd.reply_to);
                            
                            if let Some(reply_to) = cmd.reply_to.clone() {
                                println!("DEBUG: Processing request and sending response to: {}", reply_to);
                                
                                let start_time = std::time::Instant::now();
                                let response = Self::process_request(&cmd, &_handlers, &_async_handlers).await;
                                println!("DEBUG: Generated response: success={}, has_result={}", response.success, response.result.is_some());
                                Self::send_response_sync(response, &reply_to);
                                println!("DEBUG: Response processing took: {:?}", start_time.elapsed());
                            }
                        },
                        Err(e) => {
                            println!("DEBUG: Failed to parse datagram: {}", e);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Non-blocking socket, no data available
                    if poll_count % 1000 == 0 {
                        println!("DEBUG: No data available, continuing poll (count {})", poll_count);
                    }
                    // Small sleep to avoid busy-waiting
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => {
                    eprintln!("Error receiving datagram: {}", e);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        // Cleanup
        let _ = fs::remove_file(&socket_path);
        println!("SOCK_DGRAM server stopped");
        Ok(())
    }

    fn process_datagram_sync(
        data: &[u8],
    ) -> Option<(JanusResponse, String)> {
        match serde_json::from_slice::<JanusRequest>(data) {
            Ok(cmd) => {
                println!("DEBUG: Received SOCK_DGRAM request: {} (ID: {})", cmd.request, cmd.id);
                println!("DEBUG: Request reply_to field: {:?}", cmd.reply_to);
                
                if let Some(reply_to) = cmd.reply_to.clone() {
                    println!("DEBUG: Processing request and sending response to: {}", reply_to);
                    
                    // Process built-in requests immediately with zero async overhead
                    println!("DEBUG: About to process request type: {}", cmd.request);
                    let response = match cmd.request.as_str() {
                        "ping" => {
                            let result = serde_json::json!({
                                "pong": true,
                                "timestamp": std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs_f64()
                            });
                            JanusResponse::success(cmd.id.clone(), Some(result))
                        },
                        "echo" => {
                            let message = cmd.args.as_ref()
                                .and_then(|args| args.get("message"))
                                .cloned()
                                .unwrap_or_else(|| serde_json::Value::String("Hello from Rust SOCK_DGRAM server!".to_string()));
                            
                            JanusResponse::success(
                                cmd.id.clone(),
                                Some(serde_json::json!({"echo": message}))
                            )
                        },
                        "get_info" => JanusResponse::success(
                            cmd.id.clone(),
                            Some(serde_json::json!({
                                "implementation": "Rust",
                                "version": "1.0.0",
                                "protocol": "SOCK_DGRAM"
                            }))
                        ),
                        "manifest" => {
                            println!("DEBUG: Processing manifest request");
                            // Return a minimal manifest
                            let manifest = serde_json::json!({
                                "version": "1.0.0",
                                "name": "Rust Janus Server API",
                                "description": "Rust implementation of Janus SOCK_DGRAM server"
                            });
                            let response = JanusResponse::success(cmd.id.clone(), Some(manifest));
                            println!("DEBUG: Created manifest response: success={}, has_result={}", response.success, response.result.is_some());
                            response
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
                            JanusResponse::success(cmd.id.clone(), Some(result))
                        },
                        "slow_process" => {
                            // For slow_process, we can't use async sleep in this sync context
                            // Return a response indicating it would take time
                            let result = serde_json::json!({
                                "processed": true,
                                "delay": "2000ms",
                                "message": cmd.args.as_ref()
                                    .and_then(|args| args.get("message"))
                                    .cloned()
                                    .unwrap_or(serde_json::Value::String("test".to_string()))
                            });
                            JanusResponse::success(cmd.id.clone(), Some(result))
                        },
                        _ => {
                            // For non-built-in requests, return error 
                            JanusResponse::error(cmd.id.clone(), crate::error::JSONRPCError::new(
                                crate::error::JSONRPCErrorCode::MethodNotFound,
                                Some(format!("Method '{}' not found", cmd.request))
                            ))
                        }
                    };
                    
                    println!("DEBUG: Generated response, returning Some((response, reply_to))");
                    Some((response, reply_to))
                } else {
                    println!("DEBUG: No reply_to field, not sending response");
                    None
                }
            }
            Err(e) => {
                eprintln!("Failed to parse datagram: {}", e);
                None
            }
        }
    }

    #[allow(dead_code)]
    async fn process_datagram(
        data: &[u8],
        _handlers: &Arc<Mutex<HashMap<String, JanusRequestHandler>>>,
        _async_handlers: &Arc<Mutex<HashMap<String, JanusAsyncRequestHandler>>>,
        _sender_address: String,
    ) {
        // Try synchronous processing first for built-in requests
        if let Some((response, reply_to)) = Self::process_datagram_sync(data) {
            let start_time = std::time::Instant::now();
            println!("DEBUG: Generated immediate response: success={}, has_result={}", response.success, response.result.is_some());
            Self::send_response_sync(response, &reply_to);
            println!("DEBUG: Response processing took: {:?}", start_time.elapsed());
        }
    }

    #[allow(dead_code)]
    async fn process_request(
        cmd: &JanusRequest,
        handlers: &Arc<Mutex<HashMap<String, JanusRequestHandler>>>,
        async_handlers: &Arc<Mutex<HashMap<String, JanusAsyncRequestHandler>>>,
    ) -> JanusResponse {
        // Check async handlers first
        let async_handlers_guard = async_handlers.lock().await;
        let response = if let Some(async_handler) = async_handlers_guard.get(&cmd.request) {
            let future = async_handler(cmd.clone());
            drop(async_handlers_guard); // Release lock before await
            match future.await {
                Ok(result) => JanusResponse::success(cmd.id.clone(), Some(result)),
                Err(e) => JanusResponse::error(cmd.id.clone(), e),
            }
        } else {
            drop(async_handlers_guard);
            
            // Check sync handlers
            let handlers_guard = handlers.lock().await;
            if let Some(handler) = handlers_guard.get(&cmd.request) {
                match handler(cmd.clone()) {
                    Ok(result) => JanusResponse::success(cmd.id.clone(), Some(result)),
                    Err(e) => JanusResponse::error(cmd.id.clone(), e),
                }
            } else {
                drop(handlers_guard);
                
                // Default handlers (matching main binary)
                match cmd.request.as_str() {
                "ping" => {
                    let result = serde_json::json!({
                        "pong": true,
                        "timestamp": std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs_f64()
                    });
                    JanusResponse::success(cmd.id.clone(), Some(result))
                },
                "echo" => {
                    let message = cmd.args.as_ref()
                        .and_then(|args| args.get("message"))
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::String("Hello from Rust SOCK_DGRAM server!".to_string()));
                    
                    JanusResponse::success(
                        cmd.id.clone(),
                        Some(serde_json::json!({"echo": message}))
                    )
                }
                "get_info" => JanusResponse::success(
                    cmd.id.clone(),
                    Some(serde_json::json!({
                        "implementation": "Rust",
                        "version": "1.0.0",
                        "protocol": "SOCK_DGRAM"
                    }))
                ),
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
                    
                    JanusResponse::success(
                        cmd.id.clone(),
                        Some(result)
                    )
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
                    
                    JanusResponse::success(
                        cmd.id.clone(),
                        Some(result)
                    )
                }
                "manifest" => {
                    println!("DEBUG: Processing manifest request");
                    // Return a minimal manifest (matching sync handler)
                    let manifest = serde_json::json!({
                        "version": "1.0.0",
                        "name": "Rust Janus Server API",
                        "description": "Rust implementation of Janus SOCK_DGRAM server"
                    });
                    let response = JanusResponse::success(cmd.id.clone(), Some(manifest));
                    println!("DEBUG: Created manifest response: success={}, has_result={}", response.success, response.result.is_some());
                    response
                }
                "test_echo" => {
                    // Handle test_echo request for high-level API tests
                    let message = cmd.args.as_ref()
                        .and_then(|args| args.get("message"))
                        .cloned()
                        .unwrap_or_else(|| serde_json::Value::String("Hello Server!".to_string()));
                    
                    JanusResponse::success(
                        cmd.id.clone(),
                        Some(serde_json::json!({"echo": message}))
                    )
                }
                _ => {
                    use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode};
                    let error = JSONRPCError::new(
                        JSONRPCErrorCode::MethodNotFound,
                        Some(format!("Request '{}' not registered", cmd.request))
                    );
                    JanusResponse::error(cmd.id.clone(), error)
                },
                }
            }
        };

        response
    }

    fn send_response_sync(response: JanusResponse, reply_to: &str) {
        println!("[RUST-SERVER] send_response_sync START - Target: {}", reply_to);
        println!("[RUST-SERVER] Response success: {}, has_result: {}", response.success, response.result.is_some());
        
        let serialize_start = std::time::Instant::now();
        match serde_json::to_vec(&response) {
            Ok(response_data) => {
                println!("[RUST-SERVER] Response serialized - {} bytes in {:?}", response_data.len(), serialize_start.elapsed());
                
                let socket_start = std::time::Instant::now();
                match std::os::unix::net::UnixDatagram::unbound() {
                    Ok(client_sock) => {
                        println!("[RUST-SERVER] Unbound socket created in {:?}", socket_start.elapsed());
                        
                        // Check if target socket exists with detailed info
                        let socket_path = std::path::Path::new(reply_to);
                        if socket_path.exists() {
                            println!("[RUST-SERVER] SUCCESS: Target socket file exists: {}", reply_to);
                            // Get file metadata
                            match std::fs::metadata(reply_to) {
                                Ok(metadata) => {
                                    println!("[RUST-SERVER] Socket file type: {:?}, permissions: {:?}", metadata.file_type(), metadata.permissions());
                                }
                                Err(e) => println!("[RUST-SERVER] Cannot read socket metadata: {}", e),
                            }
                        } else {
                            println!("[RUST-SERVER] WARNING: Target socket file does NOT exist: {}", reply_to);
                            // Check if directory exists
                            if let Some(parent) = socket_path.parent() {
                                if parent.exists() {
                                    println!("[RUST-SERVER] Parent directory exists: {:?}", parent);
                                    // List files in directory
                                    match std::fs::read_dir(parent) {
                                        Ok(entries) => {
                                            println!("[RUST-SERVER] Files in /tmp:");
                                            for entry in entries {
                                                if let Ok(entry) = entry {
                                                    let name = entry.file_name();
                                                    if name.to_string_lossy().contains("janus_manifest") {
                                                        println!("[RUST-SERVER]   Found janus_manifest file: {:?}", name);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => println!("[RUST-SERVER] Cannot list directory: {}", e),
                                    }
                                } else {
                                    println!("[RUST-SERVER] Parent directory does NOT exist: {:?}", parent);
                                }
                            }
                        }
                        
                        let send_start = std::time::Instant::now();
                        // Try sending with a few quick retries to handle race conditions
                        let mut sent = false;
                        for attempt in 1..=5 {
                            println!("[RUST-SERVER] Send attempt {} to {}", attempt, reply_to);
                            
                            // Check if target file exists before each attempt
                            let file_exists = std::path::Path::new(reply_to).exists();
                            println!("[RUST-SERVER] Target file exists before attempt {}: {}", attempt, file_exists);
                            
                            match client_sock.send_to(&response_data, reply_to) {
                                Ok(bytes_sent) => {
                                    println!("[RUST-SERVER] SUCCESS: Sent {} bytes in {:?} (attempt {})", bytes_sent, send_start.elapsed(), attempt);
                                    println!("[RUST-SERVER] Response successfully sent to: {}", reply_to);
                                    
                                    // Check if file still exists after successful send
                                    let file_exists_after = std::path::Path::new(reply_to).exists();
                                    println!("[RUST-SERVER] Target file exists after successful send: {}", file_exists_after);
                                    
                                    sent = true;
                                    break;
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                    println!("[RUST-SERVER] WARNING: Socket not found on attempt {}: {} (error: {})", attempt, reply_to, e);
                                    
                                    // Check file existence after NotFound error
                                    let file_exists_after_error = std::path::Path::new(reply_to).exists();
                                    println!("[RUST-SERVER] Target file exists after NotFound error: {}", file_exists_after_error);
                                    
                                    if attempt < 5 {
                                        println!("[RUST-SERVER] Retrying after 1 millisecond...");
                                        std::thread::sleep(std::time::Duration::from_millis(1));
                                    }
                                    continue;
                                }
                                Err(e) => {
                                    println!("[RUST-SERVER] ERROR: Send failed on attempt {}: {} (error: {})", attempt, reply_to, e);
                                    println!("[RUST-SERVER] Send took: {:?} (FAILED attempt {})", send_start.elapsed(), attempt);
                                    eprintln!("Error sending response to {}: {}", reply_to, e);
                                    break;
                                }
                            }
                        }
                        
                        // Check final result
                        if !sent {
                            println!("[RUST-SERVER] FINAL ERROR: Failed to send response after all attempts to: {}", reply_to);
                            let final_file_exists = std::path::Path::new(reply_to).exists();
                            println!("[RUST-SERVER] Target file exists at final failure: {}", final_file_exists);
                        } else {
                            println!("[RUST-SERVER] RESPONSE SEND COMPLETED SUCCESSFULLY to: {}", reply_to);
                        }
                    }
                    Err(e) => {
                        println!("DEBUG: Socket creation took: {:?} (FAILED)", socket_start.elapsed());
                        eprintln!("Failed to create unbound socket for response: {} (kind: {:?})", e, e.kind());
                    }
                }
            }
            Err(e) => {
                println!("DEBUG: Serialization took: {:?} (FAILED)", serialize_start.elapsed());
                eprintln!("Error serializing response: {}", e);
            }
        }
    }

    #[allow(dead_code)]
    async fn send_response(response: JanusResponse, reply_to: &str) {
        Self::send_response_sync(response, reply_to);
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