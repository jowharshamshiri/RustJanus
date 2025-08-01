use clap::{Arg, Command};
use serde_json;
use std::os::unix::net::UnixDatagram;
// Note: std::path::Path not needed in current SOCK_DGRAM implementation
use std::fs;
use rust_janus::specification::{ApiSpecification, ApiSpecificationParser};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{mpsc, Mutex, Arc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Removed chrono import - using std::time for Unix timestamps

// Event system types
type EventHandler = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

#[derive(Debug)]
struct ServerConfig {
    socket_path: String,
    max_connections: usize,
    default_timeout: u64,
    max_message_size: usize,
    cleanup_on_start: bool,
    cleanup_on_shutdown: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            socket_path: String::new(),
            max_connections: 100,
            default_timeout: 30,
            max_message_size: 65536,
            cleanup_on_start: true,
            cleanup_on_shutdown: true,
        }
    }
}

#[derive(Default)]
struct EventEmitter {
    listening_handlers: Arc<Mutex<Vec<EventHandler>>>,
    connection_handlers: Arc<Mutex<Vec<EventHandler>>>,
    disconnection_handlers: Arc<Mutex<Vec<EventHandler>>>,
    command_handlers: Arc<Mutex<Vec<EventHandler>>>,
    response_handlers: Arc<Mutex<Vec<EventHandler>>>,
    error_handlers: Arc<Mutex<Vec<EventHandler>>>,
}

impl EventEmitter {
    fn new() -> Self {
        Self::default()
    }
    
    fn on(&self, event_type: &str, handler: EventHandler) {
        let handlers = match event_type {
            "listening" => &self.listening_handlers,
            "connection" => &self.connection_handlers,
            "disconnection" => &self.disconnection_handlers,
            "command" => &self.command_handlers,
            "response" => &self.response_handlers,
            "error" => &self.error_handlers,
            _ => return,
        };
        
        if let Ok(mut handlers_vec) = handlers.lock() {
            handlers_vec.push(handler);
        }
    }
    
    fn emit(&self, event_type: &str, data: serde_json::Value) {
        let handlers_clone = match event_type {
            "listening" => {
                if let Ok(handlers) = self.listening_handlers.lock() {
                    handlers.clone()
                } else {
                    return;
                }
            }
            "connection" => {
                if let Ok(handlers) = self.connection_handlers.lock() {
                    handlers.clone()
                } else {
                    return;
                }
            }
            "disconnection" => {
                if let Ok(handlers) = self.disconnection_handlers.lock() {
                    handlers.clone()
                } else {
                    return;
                }
            }
            "command" => {
                if let Ok(handlers) = self.command_handlers.lock() {
                    handlers.clone()
                } else {
                    return;
                }
            }
            "response" => {
                if let Ok(handlers) = self.response_handlers.lock() {
                    handlers.clone()
                } else {
                    return;
                }
            }
            "error" => {
                if let Ok(handlers) = self.error_handlers.lock() {
                    handlers.clone()
                } else {
                    return;
                }
            }
            _ => return,
        };
        
        for handler in handlers_clone {
            let data_clone = data.clone();
            thread::spawn(move || {
                handler(data_clone);
            });
        }
    }
    
    fn cleanup_socket_file(socket_path: &str) -> Result<(), std::io::Error> {
        if std::path::Path::new(socket_path).exists() {
            std::fs::remove_file(socket_path)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ClientConnection {
    id: String,
    address: String,
    created_at: SystemTime,
    last_activity: SystemTime,
    message_count: u32,
}

#[derive(Debug)]
struct ServerState {
    clients: Arc<Mutex<HashMap<String, ClientConnection>>>,
    client_id_counter: Arc<Mutex<u32>>,
}

impl ServerState {
    fn new() -> Self {
        ServerState {
            clients: Arc::new(Mutex::new(HashMap::new())),
            client_id_counter: Arc::new(Mutex::new(0)),
        }
    }

    fn add_client(&self, addr: String) -> String {
        let mut clients = self.clients.lock().unwrap();
        
        // Look for existing client by address
        for client in clients.values_mut() {
            if client.address == addr {
                client.last_activity = SystemTime::now();
                client.message_count += 1;
                return client.id.clone();
            }
        }
        
        // Create new client
        let mut counter = self.client_id_counter.lock().unwrap();
        *counter += 1;
        let client_id = format!("client-{}", *counter);
        
        let client = ClientConnection {
            id: client_id.clone(),
            address: addr,
            created_at: SystemTime::now(),
            last_activity: SystemTime::now(),
            message_count: 1,
        };
        
        clients.insert(client_id.clone(), client);
        client_id
    }

    fn get_client_count(&self) -> usize {
        let clients = self.clients.lock().unwrap();
        clients.len()
    }

    fn get_client_info(&self, client_id: &str) -> Option<ClientConnection> {
        let clients = self.clients.lock().unwrap();
        clients.get(client_id).cloned()
    }

    fn get_all_clients(&self) -> HashMap<String, ClientConnection> {
        let clients = self.clients.lock().unwrap();
        clients.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SocketCommand {
    id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    command: String,
    #[serde(rename = "reply_to")]
    reply_to: Option<String>,
    args: Option<HashMap<String, serde_json::Value>>,
    timeout: Option<f64>,
    timestamp: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocketResponse {
    #[serde(rename = "commandId")]
    command_id: String,
    #[serde(rename = "channelId")]
    channel_id: String,
    success: bool,
    result: Option<HashMap<String, serde_json::Value>>,
    error: Option<SocketError>,
    timestamp: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SocketError {
    code: String,
    message: String,
    details: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("janus")
        .about("Unified SOCK_DGRAM Unix Socket Process")
        .arg(
            Arg::new("socket")
                .long("socket")
                .value_name("PATH")
                .help("Unix socket path")
                .default_value("/tmp/rust-janus.sock"),
        )
        .arg(
            Arg::new("listen")
                .long("listen")
                .help("Listen for datagrams on socket")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("send-to")
                .long("send-to")
                .value_name("PATH")
                .help("Send datagram to socket path"),
        )
        .arg(
            Arg::new("command")
                .long("command")
                .value_name("CMD")
                .help("Command to send")
                .default_value("ping"),
        )
        .arg(
            Arg::new("message")
                .long("message")
                .value_name("MSG")
                .help("Message to send")
                .default_value("hello"),
        )
        .arg(
            Arg::new("spec")
                .long("spec")
                .value_name("PATH")
                .help("API specification file (required for validation)"),
        )
        .arg(
            Arg::new("channel")
                .long("channel")
                .value_name("CHANNEL")
                .help("Channel ID for command routing")
                .default_value("test"),
        )
        .get_matches();

    let socket_path = matches.get_one::<String>("socket").unwrap();
    
    // Load API specification if provided
    let api_spec = if let Some(spec_path) = matches.get_one::<String>("spec") {
        match fs::read_to_string(spec_path) {
            Ok(spec_content) => {
                match ApiSpecificationParser::from_json(&spec_content) {
                    Ok(spec) => {
                        println!("Loaded API specification v{}", spec.version);
                        Some(spec)
                    }
                    Err(e) => {
                        eprintln!("Failed to parse API specification: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read API specification file: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    if matches.get_flag("listen") {
        listen_for_datagrams(socket_path, api_spec).await
    } else if let Some(target) = matches.get_one::<String>("send-to") {
        let command = matches.get_one::<String>("command").unwrap();
        let message = matches.get_one::<String>("message").unwrap();
        send_datagram(target, command, message).await
    } else {
        eprintln!("Usage: either --listen or --send-to required");
        std::process::exit(1);
    }
}

async fn listen_for_datagrams(socket_path: &str, api_spec: Option<ApiSpecification>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listening for SOCK_DGRAM on: {}", socket_path);
    
    // Initialize server config and event system
    let config = ServerConfig {
        socket_path: socket_path.to_string(),
        ..Default::default()
    };
    let events = Arc::new(EventEmitter::new());
    
    // Cleanup existing socket if configured
    if config.cleanup_on_start {
        if let Err(e) = EventEmitter::cleanup_socket_file(socket_path) {
            let error_data = serde_json::json!({
                "error": format!("Failed to cleanup socket file: {}", e)
            });
            events.emit("error", error_data);
            return Err(Box::new(e));
        }
    }
    
    let socket = UnixDatagram::bind(socket_path)?;
    
    // Initialize server state for client tracking
    let server_state = Arc::new(ServerState::new());
    
    // Emit listening event
    events.emit("listening", serde_json::json!({}));
    
    // Set up signal handling with socket cleanup
    let socket_path_clone = socket_path.to_string();
    let events_clone = Arc::clone(&events);
    let config_cleanup = config.cleanup_on_shutdown;
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("\nShutting down server...");
        
        if config_cleanup {
            if let Err(e) = EventEmitter::cleanup_socket_file(&socket_path_clone) {
                let error_data = serde_json::json!({
                    "error": format!("Failed to cleanup socket file: {}", e)
                });
                events_clone.emit("error", error_data);
            } else {
                println!("Socket cleaned up");
            }
        }
        std::process::exit(0);
    });
    
    // Ensure cleanup on drop
    let _cleanup_guard = SocketCleanupGuard::new(socket_path);
    
    println!("Ready to receive datagrams");
    
    loop {
        let mut buffer = vec![0u8; 64 * 1024];
        let (size, addr) = socket.recv_from(&mut buffer)?;
        
        let data = &buffer[..size];
        
        match serde_json::from_slice::<SocketCommand>(data) {
            Ok(cmd) => {
                // Track client activity
                let client_id = server_state.add_client(format!("{:?}", addr));
                
                println!("Received datagram: {} (ID: {}) from client {} [Total clients: {}]", 
                    cmd.command, cmd.id, client_id, server_state.get_client_count());
                
                // Emit command event
                let command_data = serde_json::json!({
                    "command": cmd,
                    "clientId": client_id
                });
                events.emit("command", command_data);
                
                // Send response via reply_to if specified
                if let Some(reply_to) = &cmd.reply_to {
                    if let Ok(response) = send_response(&cmd.id, &cmd.channel_id, &cmd.command, &cmd.args, reply_to, &api_spec, &server_state, &client_id) {
                        // Emit response event
                        let response_data = serde_json::json!({
                            "response": response,
                            "clientId": client_id
                        });
                        events.emit("response", response_data);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse datagram: {}", e);
                // Emit error event
                let error_data = serde_json::json!({
                    "error": format!("Failed to parse datagram: {}", e)
                });
                events.emit("error", error_data);
            }
        }
    }
}

// RAII guard for socket cleanup
struct SocketCleanupGuard {
    socket_path: String,
}

impl SocketCleanupGuard {
    fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
        }
    }
}

impl Drop for SocketCleanupGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket_path);
    }
}

async fn send_datagram(target_socket: &str, command: &str, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Sending SOCK_DGRAM to: {}", target_socket);
    
    // Create response socket path
    let response_socket = format!("/tmp/rust-response-{}.sock", std::process::id());
    
    // Remove existing response socket
    let _ = fs::remove_file(&response_socket);
    
    // Create response socket for receiving reply
    let response_sock = UnixDatagram::bind(&response_socket)?;
    
    let mut args = HashMap::new();
    args.insert("message".to_string(), serde_json::Value::String(message.to_string()));
    
    let cmd = SocketCommand {
        id: generate_id(),
        channel_id: "test".to_string(),
        command: command.to_string(),
        reply_to: Some(response_socket.clone()),
        args: Some(args),
        timeout: Some(5.0),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
    };
    
    let cmd_data = serde_json::to_vec(&cmd)?;
    
    // Send datagram to target
    let client_sock = UnixDatagram::unbound()?;
    client_sock.send_to(&cmd_data, target_socket)?;
    
    // Wait for response
    let mut buffer = vec![0u8; 64 * 1024];
    let (size, _) = response_sock.recv_from(&mut buffer)?;
    
    let response_data = &buffer[..size];
    match serde_json::from_slice::<SocketResponse>(response_data) {
        Ok(response) => {
            println!("Response: Success={}, Result={:?}", response.success, response.result);
        }
        Err(e) => {
            eprintln!("Failed to parse response: {}", e);
        }
    }
    
    // Cleanup
    let _ = fs::remove_file(&response_socket);
    
    Ok(())
}

// CommandHandler type for command execution with timeout
type CommandResult = Result<std::collections::HashMap<String, serde_json::Value>, String>;
type CommandHandler = Box<dyn Fn(&Option<std::collections::HashMap<String, serde_json::Value>>) -> CommandResult + Send>;

// Execute a command handler with timeout
fn execute_with_timeout(
    handler: CommandHandler,
    args: &Option<std::collections::HashMap<String, serde_json::Value>>,
    timeout_seconds: u64,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let (tx, rx) = mpsc::channel();
    let args_clone = args.clone();
    
    thread::spawn(move || {
        let result = handler(&args_clone);
        let _ = tx.send(result);
    });
    
    match rx.recv_timeout(Duration::from_secs(timeout_seconds)) {
        Ok(result) => result.map_err(|e| e),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(format!("Handler execution timed out after {} seconds", timeout_seconds))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("Handler thread disconnected".to_string())
        }
    }
}

// Get built-in command handler
fn get_builtin_command_handler(
    command: &str,
    api_spec: &Option<ApiSpecification>,
    server_state: &Arc<ServerState>,
    client_id: &str,
) -> Option<CommandHandler> {
    match command {
        "spec" => {
            let spec_clone = api_spec.clone();
            Some(Box::new(move |_args| {
                if let Some(spec) = &spec_clone {
                    match serde_json::to_value(spec) {
                        Ok(spec_value) => {
                            let mut result = std::collections::HashMap::new();
                            result.insert("specification".to_string(), spec_value);
                            Ok(result)
                        }
                        Err(e) => Err(format!("Failed to serialize API specification: {}", e))
                    }
                } else {
                    Err("No API specification loaded on server".to_string())
                }
            }))
        }
        "ping" => {
            Some(Box::new(|args| {
                let mut result = std::collections::HashMap::new();
                result.insert("pong".to_string(), serde_json::Value::Bool(true));
                match serde_json::to_value(args) {
                    Ok(echo_value) => result.insert("echo".to_string(), echo_value),
                    Err(_) => result.insert("echo".to_string(), serde_json::Value::Null),
                };
                Ok(result)
            }))
        }
        "echo" => {
            Some(Box::new(|args| {
                let mut result = std::collections::HashMap::new();
                if let Some(args) = args {
                    if let Some(msg) = args.get("message") {
                        result.insert("echo".to_string(), msg.clone());
                    }
                }
                Ok(result)
            }))
        }
        "get_info" => {
            let state_clone = server_state.clone();
            let client_id_clone = client_id.to_string();
            Some(Box::new(move |_args| {
                let mut result = std::collections::HashMap::new();
                result.insert("implementation".to_string(), serde_json::Value::String("Rust".to_string()));
                result.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));
                result.insert("protocol".to_string(), serde_json::Value::String("SOCK_DGRAM".to_string()));
                result.insert("client_count".to_string(), serde_json::Value::Number(serde_json::Number::from(state_clone.get_client_count())));
                result.insert("client_id".to_string(), serde_json::Value::String(client_id_clone.clone()));
                Ok(result)
            }))
        }
        "server_stats" => {
            let state_clone = server_state.clone();
            Some(Box::new(move |_args| {
                let clients = state_clone.get_all_clients();
                let mut client_stats = Vec::new();
                
                for client in clients.values() {
                    let mut client_info = std::collections::HashMap::new();
                    client_info.insert("id".to_string(), serde_json::Value::String(client.id.clone()));
                    client_info.insert("address".to_string(), serde_json::Value::String(client.address.clone()));
                    client_info.insert("created_at".to_string(), serde_json::Value::Number(
                        serde_json::Number::from(client.created_at.duration_since(UNIX_EPOCH).unwrap().as_secs())
                    ));
                    client_info.insert("last_activity".to_string(), serde_json::Value::Number(
                        serde_json::Number::from(client.last_activity.duration_since(UNIX_EPOCH).unwrap().as_secs())
                    ));
                    client_info.insert("message_count".to_string(), serde_json::Value::Number(
                        serde_json::Number::from(client.message_count)
                    ));
                    client_stats.push(serde_json::Value::Object(
                        client_info.into_iter().collect()
                    ));
                }
                
                let mut server_info = std::collections::HashMap::new();
                server_info.insert("implementation".to_string(), serde_json::Value::String("Rust".to_string()));
                server_info.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));
                server_info.insert("protocol".to_string(), serde_json::Value::String("SOCK_DGRAM".to_string()));
                
                let mut result = std::collections::HashMap::new();
                result.insert("total_clients".to_string(), serde_json::Value::Number(serde_json::Number::from(clients.len())));
                result.insert("clients".to_string(), serde_json::Value::Array(client_stats));
                result.insert("server_info".to_string(), serde_json::Value::Object(
                    server_info.into_iter().collect()
                ));
                Ok(result)
            }))
        }
        "validate" => {
            Some(Box::new(|args| {
                let mut result = std::collections::HashMap::new();
                if let Some(args) = args {
                    if let Some(message) = args.get("message") {
                        if let Some(message_str) = message.as_str() {
                            match serde_json::from_str::<serde_json::Value>(message_str) {
                                Ok(json_data) => {
                                    result.insert("valid".to_string(), serde_json::Value::Bool(true));
                                    result.insert("data".to_string(), json_data);
                                }
                                Err(e) => {
                                    result.insert("valid".to_string(), serde_json::Value::Bool(false));
                                    result.insert("error".to_string(), serde_json::Value::String("Invalid JSON format".to_string()));
                                    result.insert("reason".to_string(), serde_json::Value::String(e.to_string()));
                                }
                            }
                        } else {
                            result.insert("valid".to_string(), serde_json::Value::Bool(false));
                            result.insert("error".to_string(), serde_json::Value::String("Message must be a string".to_string()));
                        }
                    } else {
                        result.insert("valid".to_string(), serde_json::Value::Bool(false));
                        result.insert("error".to_string(), serde_json::Value::String("No message provided for validation".to_string()));
                    }
                } else {
                    result.insert("valid".to_string(), serde_json::Value::Bool(false));
                    result.insert("error".to_string(), serde_json::Value::String("No arguments provided".to_string()));
                }
                Ok(result)
            }))
        }
        "slow_process" => {
            Some(Box::new(|args| {
                // Simulate a slow process that might timeout
                thread::sleep(Duration::from_secs(2)); // 2 second delay
                let mut result = std::collections::HashMap::new();
                result.insert("processed".to_string(), serde_json::Value::Bool(true));
                result.insert("delay".to_string(), serde_json::Value::String("2000ms".to_string()));
                if let Some(args) = args {
                    if let Some(message) = args.get("message") {
                        result.insert("message".to_string(), message.clone());
                    }
                }
                Ok(result)
            }))
        }
        _ => None,
    }
}

fn send_response(
    cmd_id: &str,
    channel_id: &str,
    command: &str,
    args: &Option<std::collections::HashMap<String, serde_json::Value>>,
    reply_to: &str,
    api_spec: &Option<ApiSpecification>,
    server_state: &Arc<ServerState>,
    client_id: &str,
) -> Result<SocketResponse, Box<dyn std::error::Error>> {
    // Built-in commands are always allowed and hardcoded (matches Go implementation exactly)
    let built_in_commands = vec!["spec", "ping", "echo", "get_info", "validate", "slow_process"];
    
    // Add arguments based on command type (matches Go implementation)
    let mut enhanced_args = args.clone().unwrap_or_default();
    if ["echo", "get_info", "validate", "slow_process"].contains(&command) {
        // These commands need a "message" argument - add default if not present
        if !enhanced_args.contains_key("message") {
            enhanced_args.insert("message".to_string(), serde_json::Value::String("test message".to_string()));
        }
    }
    // spec and ping commands don't need message arguments
    
    // Validate command against API specification if provided
    // Built-in commands bypass API spec validation
    let (mut result, mut success) = if let Some(spec) = api_spec {
        if !built_in_commands.contains(&command) {
            // Check if command exists in the API specification
            if let Some(channel) = spec.channels.get(channel_id) {
                if channel.commands.contains_key(command) {
                    // Command exists, validation would happen here
                    (None, true)
                } else {
                    // Command not found in API spec
                    let mut error_result = std::collections::HashMap::new();
                    error_result.insert("error".to_string(), serde_json::Value::String(format!("Command '{}' not found in channel '{}'", command, channel_id)));
                    (Some(error_result), false)
                }
            } else {
                // Channel not found
                let mut error_result = std::collections::HashMap::new();
                error_result.insert("error".to_string(), serde_json::Value::String(format!("Channel '{}' not found", channel_id)));
                (Some(error_result), false)
            }
        } else {
            // Built-in command, skip API spec validation
            (None, true)
        }
    } else {
        // No API spec loaded, allow all commands
        (None, true)
    };
    
    // Only process command if validation passed
    if success && result.is_none() {
        // Get built-in command handler
        if let Some(handler) = get_builtin_command_handler(command, api_spec, server_state, client_id) {
            // Execute with timeout (default 30 seconds for built-in commands)
            let timeout_seconds = if command == "slow_process" { 5 } else { 30 };
            
            match execute_with_timeout(handler, &Some(enhanced_args), timeout_seconds) {
                Ok(handler_result) => {
                    result = Some(handler_result);
                    success = true;
                }
                Err(e) => {
                    let error_code = if e.to_string().contains("timed out") {
                        "HANDLER_TIMEOUT"
                    } else {
                        "HANDLER_ERROR"
                    };
                    let mut error_result = std::collections::HashMap::new();
                    error_result.insert("error".to_string(), serde_json::Value::String(e.to_string()));
                    error_result.insert("code".to_string(), serde_json::Value::String(error_code.to_string()));
                    result = Some(error_result);
                    success = false;
                }
            }
        } else {
            let mut error_result = std::collections::HashMap::new();
            error_result.insert("error".to_string(), serde_json::Value::String(format!("Unknown command: {}", command)));
            result = Some(error_result);
            success = false;
        }
    }
    
    let response = SocketResponse {
        command_id: cmd_id.to_string(),
        channel_id: channel_id.to_string(),
        success,
        result,
        error: None,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
    };
    
    let response_data = serde_json::to_vec(&response)?;
    
    // Send response datagram to reply_to socket
    let reply_socket = UnixDatagram::unbound()?;
    reply_socket.send_to(&response_data, reply_to)?;
    
    Ok(response)
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}