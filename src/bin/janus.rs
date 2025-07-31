use clap::{Arg, Command};
use serde_json;
use std::os::unix::net::UnixDatagram;
// Note: std::path::Path not needed in current SOCK_DGRAM implementation
use std::fs;
use rust_janus::specification::{ApiSpecification, ApiSpecificationParser};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
// Removed chrono import - using std::time for Unix timestamps

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
    
    // Remove existing socket
    let _ = fs::remove_file(socket_path);
    
    let socket = UnixDatagram::bind(socket_path)?;
    
    // Set up signal handling
    let socket_path_clone = socket_path.to_string();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("\nShutting down server...");
        let _ = fs::remove_file(&socket_path_clone);
        println!("Socket cleaned up");
        std::process::exit(0);
    });
    
    // Ensure cleanup on drop
    let _cleanup_guard = SocketCleanupGuard::new(socket_path);
    
    println!("Ready to receive datagrams");
    
    loop {
        let mut buffer = vec![0u8; 64 * 1024];
        let (size, _) = socket.recv_from(&mut buffer)?;
        
        let data = &buffer[..size];
        
        match serde_json::from_slice::<SocketCommand>(data) {
            Ok(cmd) => {
                println!("Received datagram: {} (ID: {})", cmd.command, cmd.id);
                
                // Send response via reply_to if specified
                if let Some(reply_to) = &cmd.reply_to {
                    send_response(&cmd.id, &cmd.channel_id, &cmd.command, &cmd.args, reply_to, &api_spec)?;
                }
            }
            Err(e) => {
                eprintln!("Failed to parse datagram: {}", e);
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

fn send_response(
    cmd_id: &str,
    channel_id: &str,
    command: &str,
    args: &Option<std::collections::HashMap<String, serde_json::Value>>,
    reply_to: &str,
    api_spec: &Option<ApiSpecification>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (result, success) = match command {
        "spec" => {
            if let Some(spec) = api_spec {
                // Return the actual loaded API specification
                match serde_json::to_value(spec) {
                    Ok(spec_value) => {
                        let mut result = std::collections::HashMap::new();
                        result.insert("specification".to_string(), spec_value);
                        (Some(result), true)
                    }
                    Err(e) => {
                        let mut result = std::collections::HashMap::new();
                        result.insert("error".to_string(), serde_json::Value::String(format!("Failed to serialize API specification: {}", e)));
                        (Some(result), false)
                    }
                }
            } else {
                let mut result = std::collections::HashMap::new();
                result.insert("error".to_string(), serde_json::Value::String("No API specification loaded on server".to_string()));
                (Some(result), false)
            }
        }
        "ping" => {
            let mut result = std::collections::HashMap::new();
            result.insert("pong".to_string(), serde_json::Value::Bool(true));
            result.insert("echo".to_string(), serde_json::to_value(args)?);
            (Some(result), true)
        }
        "echo" => {
            let mut result = std::collections::HashMap::new();
            if let Some(args) = args {
                if let Some(msg) = args.get("message") {
                    result.insert("message".to_string(), msg.clone());
                }
            }
            (Some(result), true)
        }
        "get_info" => {
            let mut result = std::collections::HashMap::new();
            result.insert("implementation".to_string(), serde_json::Value::String("Rust".to_string()));
            result.insert("version".to_string(), serde_json::Value::String("1.0.0".to_string()));
            result.insert("protocol".to_string(), serde_json::Value::String("SOCK_DGRAM".to_string()));
            (Some(result), true)
        }
        "validate" => {
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
            (Some(result), true)
        }
        "slow_process" => {
            // Simulate a slow process that might timeout
            std::thread::sleep(std::time::Duration::from_secs(2)); // 2 second delay
            let mut result = std::collections::HashMap::new();
            result.insert("processed".to_string(), serde_json::Value::Bool(true));
            result.insert("delay".to_string(), serde_json::Value::String("2000ms".to_string()));
            if let Some(args) = args {
                if let Some(message) = args.get("message") {
                    result.insert("message".to_string(), message.clone());
                }
            }
            (Some(result), true)
        }
        _ => {
            let mut result = std::collections::HashMap::new();
            result.insert("error".to_string(), serde_json::Value::String(format!("Unknown command: {}", command)));
            (Some(result), false)
        }
    };
    
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
    
    Ok(())
}

fn generate_id() -> String {
    uuid::Uuid::new_v4().to_string()
}