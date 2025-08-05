# RustJanus

A production-ready Unix domain socket communication library for Rust with **SOCK_DGRAM connectionless communication** and automatic ID management.

## Features

- **Connectionless SOCK_DGRAM**: Unix domain datagram sockets with reply-to mechanism
- **Automatic ID Management**: RequestHandle system hides UUID complexity from users
- **Async-First Architecture**: Tokio-based async patterns for non-blocking operations
- **Cross-Language Compatibility**: Perfect compatibility with Go, Swift, and TypeScript implementations
- **Dynamic Specification**: Server-provided Manifests with auto-fetch validation
- **Security Framework**: 27 comprehensive security mechanisms and attack prevention
- **JSON-RPC 2.0 Compliance**: Standardized error codes and response format
- **Memory Safety**: Rust's ownership system with zero-cost abstractions
- **Production Ready**: Enterprise-grade error handling and resource management
- **Cross-Platform**: Works on all Unix-like systems (Linux, macOS, BSD)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
RustJanus = "0.1"
```

### Simple Client Example

```rust
use rust_janus::protocol::JanusClient;
use rust_janus::config::JanusClientConfig;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with automatic Manifest fetching
    let mut client = JanusClient::new(
        "/tmp/my_socket.sock".to_string(),
        "my_channel".to_string(),
        JanusClientConfig::default(),
    ).await?;
    
    // Send command - ID management is automatic
    let mut args = HashMap::new();
    args.insert("message".to_string(), serde_json::json!("Hello World"));
    
    let response = client.send_command("echo", Some(args), None).await?;
    println!("Response: {:?}", response);
    Ok(())
}
```

### Advanced Request Tracking

```rust
use rust_janus::protocol::{JanusClient, RequestHandle};
use rust_janus::config::JanusClientConfig;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = JanusClient::new(
        "/tmp/my_socket.sock".to_string(),
        "my_channel".to_string(),
        JanusClientConfig::default(),
    ).await?;
    
    let mut args = HashMap::new();
    args.insert("data".to_string(), serde_json::json!("processing_task"));
    
    // Send command with RequestHandle for tracking
    let (handle, response_rx) = client.send_command_with_handle(
        "process_data",
        Some(args),
        Some(Duration::from_secs(30)),
    ).await?;
    
    println!("Request started: {} on channel {}", 
        handle.get_command(), handle.get_channel());
    
    // Can check status or cancel if needed
    if handle.is_cancelled() {
        println!("Request was cancelled");
        return Ok(());
    }
    
    // Wait for response
    match response_rx.await {
        Ok(Ok(response)) => println!("Success: {:?}", response),
        Ok(Err(err)) => println!("Error: {:?}", err),
        Err(_) => {
            client.cancel_request(&handle)?;
            println!("Request cancelled due to timeout");
        }
    }
    
    Ok(())
}
```

### Server with Command Handlers

```rust
use rust_janus::server::JanusServer;
use rust_janus::protocol::message_types::JanusCommand;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = JanusServer::new(
        "/tmp/my_socket.sock".to_string(),
        ServerConfig::default(),
    ).await?;
    
    // Register command handlers - returns direct values
    server.register_handler("echo", |cmd: JanusCommand| async move {
        let message = cmd.args
            .as_ref()
            .and_then(|args| args.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");
        
        // Return direct value - no dictionary wrapping needed
        Ok(serde_json::json!({
            "echo": message,
            "timestamp": cmd.timestamp,
        }))
    });
    
    // Register async handler
    server.register_handler("process_data", |cmd: JanusCommand| async move {
        let data = cmd.args
            .as_ref()
            .and_then(|args| args.get("data"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        
        // Simulate async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(serde_json::json!({
            "result": format!("Processed: {}", data),
            "processed_at": chrono::Utc::now().timestamp(),
        }))
    });
    
    println!("Server listening on /tmp/my_socket.sock...");
    server.start_listening().await?;
    Ok(())
}
```

### Fire-and-Forget Commands

```rust
// Send command without waiting for response
let mut args = HashMap::new();
args.insert("event".to_string(), serde_json::json!("user_login"));
args.insert("user_id".to_string(), serde_json::json!("12345"));

match client.send_command_no_response("log_event", Some(args)).await {
    Ok(_) => println!("Event logged successfully"),
    Err(err) => println!("Failed to log event: {:?}", err),
}
```

## RequestHandle Management

```rust
use rust_janus::protocol::{JanusClient, RequestStatus};

// Get all pending requests
let handles = client.get_pending_requests();
println!("Pending requests: {}", handles.len());

for handle in &handles {
    println!("Request: {} on {} (created: {:?})", 
        handle.get_command(), 
        handle.get_channel(), 
        handle.get_timestamp());
    
    // Check status
    let status = client.get_request_status(handle);
    match status {
        RequestStatus::Pending => println!("Status: Still processing"),
        RequestStatus::Completed => println!("Status: Completed"),
        RequestStatus::Cancelled => println!("Status: Cancelled"),
    }
}

// Cancel all pending requests
let cancelled = client.cancel_all_requests();
println!("Cancelled {} requests", cancelled);
```

## Configuration

```rust
use rust_janus::config::JanusClientConfig;
use std::time::Duration;

let config = JanusClientConfig {
    max_message_size: 10 * 1024 * 1024, // 10MB
    default_timeout: Duration::from_secs(30),
    datagram_timeout: Duration::from_secs(5),
    enable_validation: true,
};

let client = JanusClient::new(
    "/tmp/my_socket.sock".to_string(),
    "my_channel".to_string(),
    config,
).await?;
```

## Error Handling

The library uses JSON-RPC 2.0 compliant error handling:

```rust
use rust_janus::error::{JSONRPCError, JSONRPCErrorCode};

match client.send_command("echo", Some(args), None).await {
    Ok(response) => println!("Success: {:?}", response),
    Err(err) => {
        match err.code {
            -32601 => println!("Command not found: {}", err.message),
            -32602 => println!("Invalid parameters: {}", err.message), 
            -32603 => println!("Internal error: {}", err.message),
            -32005 => println!("Validation failed: {}", err.message),
            _ => println!("Error {}: {}", err.code, err.message),
        }
    }
}
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