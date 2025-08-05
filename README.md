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

### API Specification (Manifest)

Before creating servers or clients, you need a Manifest file defining your API:

**my-api-spec.json:**
```json
{
  "name": "My Application API",
  "version": "1.0.0",
  "description": "Example API for demonstration",
  "channels": {
    "default": {
      "commands": {
        "get_user": {
          "description": "Retrieve user information",
          "arguments": {
            "user_id": {
              "type": "string",
              "required": true,
              "description": "User identifier"
            }
          },
          "response": {
            "type": "object",
            "properties": {
              "id": {"type": "string"},
              "name": {"type": "string"},
              "email": {"type": "string"}
            }
          }
        },
        "update_profile": {
          "description": "Update user profile",
          "arguments": {
            "user_id": {"type": "string", "required": true},
            "name": {"type": "string", "required": false},
            "email": {"type": "string", "required": false}
          },
          "response": {
            "type": "object",
            "properties": {
              "success": {"type": "boolean"},
              "updated_fields": {"type": "array"}
            }
          }
        }
      }
    }
  }
}
```

**Note**: Built-in commands (`ping`, `echo`, `get_info`, `validate`, `slow_process`, `spec`) are always available and cannot be overridden in Manifests.

### Simple Client Example

```rust
use rust_janus::{JanusClient, JSONRPCError, JSONRPCErrorCode};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client - specification is fetched automatically from server
    let client = JanusClient::new("/tmp/my-server.sock", "default").await?;
    
    // Built-in commands (always available) 
    let response = client.send_command("ping", None).await?;
    if response.success {
        println!("Server ping: {:?}", response.result);
    }
    
    // Custom command defined in Manifest (arguments validated automatically)
    let mut user_args = HashMap::new();
    user_args.insert("user_id".to_string(), serde_json::json!("user123"));
    
    let response = client.send_command("get_user", Some(user_args)).await?;
    if response.success {
        println!("User data: {:?}", response.result);
    } else {
        println!("Error: {:?}", response.error);
    }
    
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

### Server Usage

```rust
use RustJanus::{JanusServer, ServerConfig, JSONRPCError, JSONRPCErrorCode};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create server with configuration
    let config = ServerConfig {
        socket_path: "/tmp/my-server.sock".to_string(),
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
        ..Default::default()
    };
    
    let mut server = JanusServer::new(config);
    
    // Register handlers for commands defined in the Manifest
    server.register_handler("get_user", |cmd| {
        // Extract user_id argument (validated by Manifest)
        let user_id = cmd.args.as_ref()
            .and_then(|args| args.get("user_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| JSONRPCError::new(
                JSONRPCErrorCode::InvalidParams, 
                "Missing user_id"
            ))?;
        
        // Simulate user lookup
        Ok(json!({
            "id": user_id,
            "name": "John Doe",
            "email": "john@example.com"
        }))
    }).await;
    
    server.register_handler("update_profile", |cmd| {
        let args = cmd.args.as_ref().ok_or_else(|| 
            JSONRPCError::new(JSONRPCErrorCode::InvalidParams, "No arguments")
        )?;
        
        let user_id = args.get("user_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JSONRPCError::new(
                JSONRPCErrorCode::InvalidParams, 
                "Missing user_id"
            ))?;
        
        let mut updated_fields = Vec::new();
        if args.contains_key("name") { updated_fields.push("name"); }
        if args.contains_key("email") { updated_fields.push("email"); }
        
        Ok(json!({
            "success": true,
            "updated_fields": updated_fields
        }))
    }).await;
    
    // Start listening (blocks until stopped)
    server.start_listening().await?;
    
    Ok(())
}
```

### Client Usage

```rust
use RustJanus::JanusClient;
use std::collections::HashMap;
use std::time::Duration;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client - specification is fetched automatically from server
    let client = JanusClient::new("/tmp/my-server.sock", "default").await?;
    
    // Built-in commands (always available)
    let response = client.send_command("ping", None).await?;
    if response.success {
        println!("Server ping: {:?}", response.result);
    }
    
    // Custom command defined in Manifest (arguments validated automatically)
    let mut user_args = HashMap::new();
    user_args.insert("user_id".to_string(), json!("user123"));
    
    let response = client.send_command("get_user", Some(user_args)).await?;
    if response.success {
        println!("User data: {:?}", response.result);
    } else {
        println!("Error: {:?}", response.error);
    }
    
    // Fire-and-forget command (no response expected)
    let mut log_args = HashMap::new();
    log_args.insert("level".to_string(), json!("info"));
    log_args.insert("message".to_string(), json!("User profile updated"));
    
    client.send_command_no_response("log_event", Some(log_args)).await?;
    
    // Get server API specification
    let spec_response = client.send_command("spec", None).await?;
    println!("Server API spec: {:?}", spec_response.result);
    
    Ok(())
}
```

### Fire-and-Forget Commands

```rust
// Send command without waiting for response
let mut log_args = HashMap::new();
log_args.insert("level".to_string(), json!("info"));
log_args.insert("message".to_string(), json!("User profile updated"));

match client.send_command_no_response("log_event", Some(log_args)).await {
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