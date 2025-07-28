# RustUnixSockAPI

A general-purpose Unix domain socket communication library for Rust.

## Features

- **High-Performance IPC**: Efficient Unix domain socket communication
- **Generic Message Handling**: Support for any serializable message type via Serde
- **JSON Protocol**: Built-in JSON serialization/deserialization
- **Custom Handlers**: User-defined message processing logic
- **Error Recovery**: Robust error handling and automatic retry mechanisms
- **Message Envelopes**: Optional message metadata (timestamps, correlation IDs, etc.)
- **Configurable**: Extensive configuration options for timeouts, retries, and limits
- **Cross-Platform**: Works on all Unix-like systems (Linux, macOS, BSD)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
RustUnixSockAPI = "0.1"
```

### Server Example

```rust
use rs_unix_sock_comms::{UnixSocketServer, MessageHandler};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyMessage {
    action: String,
    data: String,
}

struct MyHandler;

impl MessageHandler<MyMessage> for MyHandler {
    fn handle_message(&mut self, message: MyMessage, client_id: &str) -> Option<String> {
        println!("Received: {:?} from {}", message, client_id);
        Some("Message received".to_string())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = UnixSocketServer::new("/tmp/my_socket.sock")?;
    let handler = MyHandler;
    server.run_with_handler(handler)?;
    Ok(())
}
```

### Client Example

```rust
use rs_unix_sock_comms::UnixSocketClient;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct MyMessage {
    action: String,
    data: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = UnixSocketClient::new("/tmp/my_socket.sock");
    
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
use rs_unix_sock_comms::{MessageEnvelope, UnixSocketClient};

let message = MyMessage { /* ... */ };
let envelope = MessageEnvelope::with_priority(message, 1);

let mut client = UnixSocketClient::new("/tmp/socket.sock");
let response = client.send_envelope(envelope)?;
```

### Custom Configuration

```rust
use rs_unix_sock_comms::{UnixSocketServer, ServerConfig, ClientConfig};

// Server configuration
let server_config = ServerConfig {
    max_connections: 50,
    connection_timeout: 10,
    auto_cleanup: true,
    ..Default::default()
};
let mut server = UnixSocketServer::with_config("/tmp/socket.sock", server_config)?;

// Client configuration
let client_config = ClientConfig {
    max_retries: 5,
    retry_delay_ms: 200,
    auto_correlation: true,
    ..Default::default()
};
let client = UnixSocketClient::with_config("/tmp/socket.sock", client_config);
```

### Built-in Message Types

The library provides several built-in message types:

```rust
use rs_unix_sock_comms::{TextMessage, CommandMessage, ResponseMessage, JsonMessage};

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
use rs_unix_sock_comms::{SocketError, Result};

fn send_message() -> Result<String> {
    let mut client = UnixSocketClient::new("/tmp/socket.sock");
    
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
use rs_unix_sock_comms::utils::{
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