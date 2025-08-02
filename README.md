# RustJanus

A production-ready Unix domain socket communication library for Rust with **async-first architecture** and cross-language compatibility.

## Features

- **Async Communication Architecture**: True async patterns with background message listeners and response correlation
- **Cross-Language Compatibility**: Seamless communication with Go and Swift implementations
- **Persistent Connections**: Efficient async connection management with proper response tracking
- **Security Framework**: Comprehensive path validation, resource limits, and attack prevention
- **Manifest Engine**: JSON/YAML-driven command validation and type safety
- **Performance Optimized**: Async patterns optimized for Unix socket inherent async nature
- **Production Ready**: Enterprise-grade error handling and resource management
- **Cross-Platform**: Works on all Unix-like systems (Linux, macOS, BSD)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
RustJanus = "0.1"
```

### Async Client Example

```rust
use rust_janus::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load Manifest
    let spec_data = std::fs::read_to_string("manifest.json")?;
    let spec = ManifestParser::from_json(&spec_data)?;
    
    // Create async client with proper configuration
    let client = JanusClient::new(
        "/tmp/my_socket.sock".to_string(),
        "my_channel".to_string(),
        spec,
        JanusClientConfig::default(),
    ).await?;
    
    // Send async command with response tracking
    let mut args = HashMap::new();
    args.insert("message".to_string(), serde_json::json!("Hello World"));
    
    let response = client.send_command(
        "echo",
        Some(args),
        Duration::from_secs(5),
        None
    ).await?;
    
    println!("Response: {:?}", response);
    Ok(())
}
```

### Client Example

```rust
use rust_janus::JanusClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyMessage {
    action: String,
    data: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = JanusClient::new("/tmp/my_socket.sock");
    
    let message = MyMessage {
        action: "test".to_string(),
        data: "Hello, Server!".to_string(),
    };
    
    let response = client.send_message(message)?;
    println!("Server response: {}", response);
    
    Ok(())
}
```

## Advanced Usage

### Message Envelopes

Use message envelopes for additional metadata:

```rust
use rust_janus::{MessageEnvelope, JanusClient};

let message = MyMessage { /* ... */ };
let envelope = MessageEnvelope::with_priority(message, 1);

let mut client = JanusClient::new("/tmp/socket.sock");
let response = client.send_envelope(envelope)?;
```

### Custom Configuration

```rust
use rust_janus::{JanusServer, ServerConfig, ClientConfig};

// Server configuration
let server_config = ServerConfig {
    max_connections: 50,
    connection_timeout: 10,
    auto_cleanup: true,
    ..Default::default()
};
let mut server = JanusServer::with_config("/tmp/socket.sock", server_config)?;

// Client configuration
let client_config = ClientConfig {
    max_retries: 5,
    retry_delay_ms: 200,
    auto_correlation: true,
    ..Default::default()
};
let client = JanusClient::with_config("/tmp/socket.sock", client_config);
```

### Built-in Message Types

The library provides several built-in message types:

```rust
use rust_janus::{TextMessage, CommandMessage, ResponseMessage, JsonMessage};

// Simple text message
let text_msg = TextMessage::new("Hello, World!");

// Command with arguments
let cmd_msg = CommandMessage::new("deploy")
    .with_args(vec!["app", "production"])
    .with_option("force", true);

// Response message
let response = ResponseMessage::success_with_data(
    "Operation completed", 
    serde_json::json!({"result": "ok"})
);

// Raw JSON message
let json_msg = JsonMessage::new(serde_json::json!({
    "custom": "data",
    "value": 42
}));
```

## Error Handling

The library provides comprehensive error handling:

```rust
use rust_janus::{SocketError, Result};

fn send_message() -> Result<String> {
    let mut client = JanusClient::new("/tmp/socket.sock");
    
    match client.send_message(message) {
        Ok(response) => Ok(response),
        Err(SocketError::Connection(_)) => {
            // Handle connection errors
            eprintln!("Failed to connect to server");
            Err(SocketError::connection("Server unavailable"))
        },
        Err(SocketError::Timeout(_)) => {
            // Handle timeout errors
            eprintln!("Request timed out");
            Err(SocketError::timeout("Server not responding"))
        },
        Err(e) => Err(e),
    }
}
```

## Utility Functions

```rust
use rust_janus::utils::{
    ensure_socket_dir, 
    cleanup_socket_file, 
    is_socket_in_use,
    unique_socket_path
};

// Ensure socket directory exists
ensure_socket_dir("/tmp/app/socket.sock")?;

// Clean up socket file
cleanup_socket_file("/tmp/old_socket.sock")?;

// Check if socket is in use
if is_socket_in_use("/tmp/socket.sock") {
    println!("Socket is already in use");
}

// Generate unique socket path
let socket_path = unique_socket_path("myapp");
```

## Examples

Run the included examples:

```bash
# Terminal 1: Start the server
cargo run --example simple_server

# Terminal 2: Run the client
cargo run --example json_client
```

## Performance

- **Low Latency**: Unix domain sockets provide the fastest IPC on Unix systems
- **High Throughput**: Efficient message serialization with minimal overhead
- **Memory Efficient**: Configurable buffer sizes and connection limits
- **Concurrent**: Support for multiple simultaneous client connections

## Testing

```bash
cargo test
```

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

## Changelog

### v0.1.0
- Initial release
- Basic Unix domain socket server and client
- JSON message protocol
- Message envelopes with metadata
- Comprehensive error handling
- Built-in message types
- Utility functions for socket management