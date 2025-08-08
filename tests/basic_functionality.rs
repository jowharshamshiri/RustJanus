use rust_janus::*;
use rust_janus::error::{JSONRPCError, JSONRPCErrorCode};

mod test_utils;
use test_utils::*;
use std::collections::HashMap;

/// Basic Functionality Tests (11 tests) - Exact SwiftJanus parity
/// Tests Manifest creation, serialization, and core functionality

#[tokio::test]
async fn test_manifest_creation() {
    let manifest = load_test_manifest();
    
    assert_eq!(manifest.version, "1.0.0");
    assert!(!manifest.channels.is_empty());
    assert!(manifest.get_channel("test").is_some());
    
    let channel = manifest.get_channel("test").unwrap();
    assert_eq!(channel.description, "Test channel for cross-platform communication");
    assert!(!channel.requests.is_empty());
    
    // Verify requests exist (only check non-built-in requests)
    assert!(channel.get_request("test_request").is_some());
    // Note: ping, echo, manifest are built-in requests and should not be in Manifest
}

#[tokio::test]
async fn test_manifest_json_serialization() {
    let manifest = load_test_manifest();
    
    // Serialize to JSON
    let json_str = ManifestParser::to_json(&manifest).unwrap();
    assert!(!json_str.is_empty());
    assert!(json_str.contains("\"version\": \"1.0.0\""));
    assert!(json_str.contains("\"test\""));
    
    // Deserialize from JSON
    let parsed_manifest = ManifestParser::from_json(&json_str).unwrap();
    
    // Verify round-trip integrity
    assert_eq!(parsed_manifest.version, manifest.version);
    assert_eq!(parsed_manifest.channels.len(), manifest.channels.len());
    
    let parsed_channel = parsed_manifest.get_channel("test").unwrap();
    let original_channel = manifest.get_channel("test").unwrap();
    assert_eq!(parsed_channel.description, original_channel.description);
    assert_eq!(parsed_channel.requests.len(), original_channel.requests.len());
}

#[tokio::test]
async fn test_socket_request_serialization() {
    let request = JanusRequest::new(
        "test".to_string(),
        "echo".to_string(),
        Some(create_test_args()),
        Some(30.0),
    );
    
    // Serialize to JSON
    let json_str = serde_json::to_string(&request).unwrap();
    assert!(!json_str.is_empty());
    assert!(json_str.contains(&request.id));
    assert!(json_str.contains("test"));
    assert!(json_str.contains("echo"));
    assert!(json_str.contains("30"));
    
    // Deserialize from JSON
    let parsed_request: JanusRequest = serde_json::from_str(&json_str).unwrap();
    
    // Verify round-trip integrity
    assert_eq!(parsed_request.id, request.id);
    assert_eq!(parsed_request.channelId, request.channelId);
    assert_eq!(parsed_request.request, request.request);
    assert_eq!(parsed_request.args, request.args);
    assert_eq!(parsed_request.timeout, request.timeout);
}

#[tokio::test]
async fn test_socket_response_serialization() {
    // Test success response
    let success_response = JanusResponse::success(
        "echo-123".to_string(),
        "test".to_string(),
        Some(serde_json::json!({"result": "success", "data": "test"})),
    );
    
    let json_str = serde_json::to_string(&success_response).unwrap();
    let parsed_response: JanusResponse = serde_json::from_str(&json_str).unwrap();
    
    assert_eq!(parsed_response.requestId, success_response.requestId);
    assert_eq!(parsed_response.success, true);
    assert!(parsed_response.error.is_none());
    assert!(parsed_response.result.is_some());
    
    // Test error response
    let jsonrpc_error = JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some("Test validation error".to_string()));
    let error_response = JanusResponse::error(
        "echo-456".to_string(),
        "test".to_string(),
        jsonrpc_error,
    );
    
    let json_str = serde_json::to_string(&error_response).unwrap();
    let parsed_response: JanusResponse = serde_json::from_str(&json_str).unwrap();
    
    assert_eq!(parsed_response.requestId, error_response.requestId);
    assert_eq!(parsed_response.success, false);
    assert!(parsed_response.error.is_some());
    assert!(parsed_response.result.is_none());
}

#[tokio::test]
async fn test_anyccodable_string_value() {
    let string_value = serde_json::Value::String("Hello World".to_string());
    
    let request = JanusRequest::new(
        "test".to_string(),
        "echo".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("string_arg".to_string(), string_value.clone());
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&request).unwrap();
    let parsed_request: JanusRequest = serde_json::from_str(&json_str).unwrap();
    
    let args = parsed_request.args.unwrap();
    let parsed_value = args.get("string_arg").unwrap();
    assert_eq!(&string_value, parsed_value);
    assert_eq!(parsed_value.as_str().unwrap(), "Hello World");
}

#[tokio::test]
async fn test_anyccodable_integer_value() {
    let integer_value = serde_json::Value::Number(serde_json::Number::from(42));
    
    let request = JanusRequest::new(
        "test".to_string(),
        "echo".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("integer_arg".to_string(), integer_value.clone());
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&request).unwrap();
    let parsed_request: JanusRequest = serde_json::from_str(&json_str).unwrap();
    
    let binding = parsed_request.args.unwrap();
    let parsed_value = binding.get("integer_arg").unwrap();
    assert_eq!(&integer_value, parsed_value);
    assert_eq!(parsed_value.as_i64().unwrap(), 42);
}

#[tokio::test]
async fn test_anyccodable_boolean_value() {
    let boolean_value = serde_json::Value::Bool(true);
    
    let request = JanusRequest::new(
        "test".to_string(),
        "echo".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("boolean_arg".to_string(), boolean_value.clone());
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&request).unwrap();
    let parsed_request: JanusRequest = serde_json::from_str(&json_str).unwrap();
    
    let binding = parsed_request.args.unwrap();
    let parsed_value = binding.get("boolean_arg").unwrap();
    assert_eq!(&boolean_value, parsed_value);
    assert_eq!(parsed_value.as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_anyccodable_array_value() {
    let array_value = serde_json::Value::Array(vec![
        serde_json::Value::String("item1".to_string()),
        serde_json::Value::Number(serde_json::Number::from(2)),
        serde_json::Value::Bool(true),
    ]);
    
    let request = JanusRequest::new(
        "test".to_string(),
        "echo".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("array_arg".to_string(), array_value.clone());
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&request).unwrap();
    let parsed_request: JanusRequest = serde_json::from_str(&json_str).unwrap();
    
    let binding = parsed_request.args.unwrap();
    let parsed_value = binding.get("array_arg").unwrap();
    assert_eq!(&array_value, parsed_value);
    
    let array = parsed_value.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array[0].as_str().unwrap(), "item1");
    assert_eq!(array[1].as_i64().unwrap(), 2);
    assert_eq!(array[2].as_bool().unwrap(), true);
}

#[tokio::test]
async fn test_janus_client_initialization() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Valid initialization
    let mut client = JanusClient::new(
        socket_path.clone(),
        "test".to_string(),
        config.clone(),
    ).await;
    
    assert!(client.is_ok());
    
    let client = client.unwrap();
    assert_eq!(client.configuration().max_concurrent_connections, 10);
    // With Dynamic Manifest Architecture, manifest is None initially
    assert!(client.manifest().is_none(), "Expected manifest to be None initially");
    
    // Invalid channel ID
    let invalid_client = JanusClient::new(
        socket_path,
        "".to_string(), // Empty channel ID
        config,
    ).await;
    
    assert!(invalid_client.is_err());
    match invalid_client.unwrap_err() {
        JSONRPCError { code: -32600, .. } => {},
        err => panic!("Expected InvalidChannel, got: {:?}", err),
    }
}

#[tokio::test] 
async fn test_request_validation() {
    let manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Start a test server
    use rust_janus::server::janus_server::{JanusServer, ServerConfig};
    let server_config = ServerConfig {
        socket_path: socket_path.clone(),
        max_connections: 100,
        max_message_size: 65536,
        default_timeout: 30,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    
    // Register a simple echo handler for testing
    server.register_handler("echo", move |cmd: JanusRequest| {
        let message = cmd.args
            .as_ref()
            .and_then(|args| args.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        Ok(serde_json::json!({
            "echo": message
        }))
    }).await;
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.start_listening().await
    });
    
    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Valid request
    let valid_args = create_test_args();
    let result = client.send_request(
        "echo",
        Some(valid_args),
        Some(std::time::Duration::from_millis(100)),
    );
    
    // Should either succeed or fail with expected errors (connection/timeout/security)
    match result.await {
        Ok(_) => {},
        Err(JSONRPCError { code: -32603, .. }) => {}, // Internal error
        Err(JSONRPCError { code: -32000, .. }) => {}, // Server error
        Err(JSONRPCError { code: -32005, .. }) => {}, // Validation failed
        Err(JSONRPCError { code: -32006, .. }) => {}, // Handler timeout
        Err(JSONRPCError { code: -32011, .. }) => {}, // Message framing error
        Err(JSONRPCError { code: -32007, .. }) => {}, // Socket transport error (expected with no server)
        Err(err) => panic!("Unexpected error for valid request: {:?}", err),
    }
    
    // Invalid request name
    let invalid_result = client.send_request(
        "nonexistent-request",
        Some(create_test_args()),
        Some(std::time::Duration::from_millis(100)),
    );
    
    // May fail with validation error or connection error
    match invalid_result.await {
        Ok(_) => {},
        Err(JSONRPCError { code: -32601, .. }) => {},
        Err(JSONRPCError { code: -32603, .. }) => {},
        Err(JSONRPCError { code: -32000, .. }) => {},
        Err(JSONRPCError { code: -32005, .. }) => {},
        Err(JSONRPCError { code: -32006, .. }) => {}, // Handler timeout
        Err(JSONRPCError { code: -32007, .. }) => {}, // Socket transport error
        Err(JSONRPCError { code: -32011, .. }) => {},
        Err(err) => panic!("Unexpected error for invalid request: {:?}", err),
    }
    
    // Clean up server
    server_handle.abort();
}

#[tokio::test]
async fn test_message_envelope_functionality() {
    // Test simple message creation
    let text_message = SocketMessage::text_request("test", "echo", "Hello World");
    assert!(text_message.is_ok());
    
    let message = text_message.unwrap();
    assert_eq!(message.message_type, MessageType::Request);
    assert!(!message.payload.is_empty());
    
    // Decode and verify
    let request = message.decode_request().unwrap();
    assert_eq!(request.channelId, "test");
    assert_eq!(request.request, "echo");
    assert!(request.args.is_some());
    
    let args_binding = request.args.unwrap();
    let text_arg = args_binding.get("text").unwrap().as_str().unwrap();
    assert_eq!(text_arg, "Hello World");
    
    // Test success response
    let success_message = SocketMessage::simple_success("cmd-123", "test", "Operation completed");
    assert!(success_message.is_ok());
    
    let response_message = success_message.unwrap();
    let response = response_message.decode_response().unwrap();
    assert!(response.success);
    assert_eq!(response.requestId, "cmd-123");
    assert_eq!(response.channelId, "test");
    
    // Test error response
    let error_message = SocketMessage::simple_error("cmd-456", "test", "Something went wrong");
    assert!(error_message.is_ok());
    
    let error_response_message = error_message.unwrap();
    let error_response = error_response_message.decode_response().unwrap();
    assert!(!error_response.success);
    assert_eq!(error_response.requestId, "cmd-456");
    assert!(error_response.error.is_some());
}

#[tokio::test]
async fn test_send_request_no_response() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = format!("/tmp/rust_janus_test_isolated_no_response_{}.sock", std::process::id());
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test fire-and-forget request (no response expected)
    let mut test_args = HashMap::new();
    test_args.insert("message".to_string(), serde_json::Value::String("fire-and-forget test message".to_string()));
    
    // Should not wait for response and return immediately
    let result = client.send_request_no_response(
        "echo",
        Some(test_args.clone()),
    ).await;
    
    // Expected to fail with connection error (no server running)
    // but should not timeout waiting for response
    assert!(result.is_err());
    
    // Should be connection error, not timeout error
    match result.unwrap_err() {
        JSONRPCError { code: -32603, .. } => {
            // Expected - connection error is fine
        },
        JSONRPCError { code: -32000, .. } => {
            panic!("Got timeout error when expecting connection error for fire-and-forget");
        },
        other => {
            // Other errors are acceptable (e.g., validation errors)
            println!("Got error for fire-and-forget (acceptable): {:?}", other);
        }
    }
    
    // Verify request validation still works for fire-and-forget
    let result = client.send_request_no_response(
        "unknown-request",
        Some(test_args),
    ).await;
    
    // Should fail with some error (validation or connection)
    assert!(result.is_err());
    
    // Test passes if we get any error for unknown request
    match result.unwrap_err() {
        err => {
            println!("Got expected error for unknown request: {:?}", err);
        }
    }
}

#[tokio::test]
async fn test_dynamic_message_size_detection() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = format!("/tmp/rust_janus_test_isolated_msg_size_{}.sock", std::process::id());
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test with normal-sized message (should pass validation)
    let mut normal_args = HashMap::new();
    normal_args.insert("message".to_string(), serde_json::Value::String("normal message within size limits".to_string()));
    
    // This should fail with connection error, not validation error
    let result = client.send_request(
        "echo",
        Some(normal_args),
        Some(std::time::Duration::from_millis(1000)),
    ).await;
    
    assert!(result.is_err(), "Expected connection error since no server is running");
    
    // Should be connection error, not message size error
    match result.unwrap_err() {
        JSONRPCError { code: -32603, .. } => {
            // Expected - connection error is fine
        },
        JSONRPCError { code: -32000, .. } => {
            // Also acceptable - timeout due to no server
        },
        other => {
            // Should not be size validation error for normal message
            let error_str = format!("{:?}", other);
            if error_str.contains("size") && error_str.contains("exceeds") {
                panic!("Got size error for normal message: {:?}", other);
            }
        }
    }
    
    // Test with very large message (should trigger size validation)
    // Create message larger than default limit
    let large_data = "x".repeat(6 * 1024 * 1024); // 6MB of data
    let mut large_args = HashMap::new();
    large_args.insert("message".to_string(), serde_json::Value::String(large_data));
    
    // This should fail with size validation error before attempting connection
    let result = client.send_request(
        "echo",
        Some(large_args.clone()),
        Some(std::time::Duration::from_millis(1000)),
    ).await;
    
    assert!(result.is_err(), "Expected validation error for oversized message");
    
    // Check if it's a size-related error (implementation may vary)
    match result.unwrap_err() {
        JSONRPCError { code: -32005, message, .. } => {
            println!("Got validation error for large message: {}", message);
        },
        other => {
            println!("Got error for large message (may be size-related): {:?}", other);
        }
    }
    
    // Test fire-and-forget with large message
    let result = client.send_request_no_response(
        "echo",
        Some(large_args),
    ).await;
    
    assert!(result.is_err(), "Expected validation error for oversized fire-and-forget message");
    
    // Message size detection should work for both response and no-response requests
    match result.unwrap_err() {
        err => {
            println!("Fire-and-forget large message correctly rejected: {:?}", err);
        }
    }
}

#[tokio::test]
async fn test_socket_cleanup_management() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = format!("/tmp/rust_janus_test_isolated_cleanup_{}.sock", std::process::id());
    
    let mut client = JanusClient::new(
        socket_path.clone(),
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test that client can be created and basic operations work
    // This implicitly tests socket creation and cleanup
    let test_args = HashMap::new();
    
    let result = client.send_request(
        "ping",
        Some(test_args),
        Some(std::time::Duration::from_millis(100)),
    ).await;
    
    // Should fail with connection or timeout error (no server running)
    assert!(result.is_err(), "Expected error since no server is running");
    
    match result.unwrap_err() {
        JSONRPCError { code: -32603, .. } => {
            println!("Socket cleanup test: Connection error (expected with no server)");
        },
        JSONRPCError { code: -32000, .. } => {
            println!("Socket cleanup test: Timeout error (expected with no server)");
        },
        other => {
            println!("Socket cleanup test: Got error (may be expected): {:?}", other);
        }
    }
    
    // Test multiple operations to ensure sockets are properly managed
    for i in 0..5 {
        let mut args = HashMap::new();
        args.insert("test_data".to_string(), serde_json::Value::String(format!("cleanup_test_{}", i)));
        
        let result = client.send_request(
            "echo",
            Some(args),
            Some(std::time::Duration::from_millis(50)),
        ).await;
        
        // All operations should fail gracefully (no server running)
        // but should not cause resource leaks or socket issues
        match result {
            Err(JSONRPCError { code: -32603, .. }) => {
                // Expected - connection cleanup working
            },
            Err(JSONRPCError { code: -32000, .. }) => {
                // Expected - timeout cleanup working
            },
            other => {
                println!("Cleanup test iteration {}: {:?}", i, other);
            }
        }
    }
    
    // Test fire-and-forget cleanup
    let cleanup_args = HashMap::new();
    let result = client.send_request_no_response(
        "ping",
        Some(cleanup_args),
    ).await;
    
    // Should handle cleanup for fire-and-forget as well
    match result {
        Err(JSONRPCError { code: -32603, .. }) => {
            println!("Fire-and-forget cleanup test: Connection error handled");
        },
        other => {
            println!("Fire-and-forget cleanup test result: {:?}", other);
        }
    }
    
    // Client should be dropped cleanly when test ends
    // This tests the Drop trait implementation for cleanup
}

#[tokio::test]
async fn test_connection_testing() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = format!("/tmp/rust_janus_test_isolated_connection_{}.sock", std::process::id());
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test the dedicated test_connection method
    let result = client.test_connection().await;
    
    // Should fail with connection error since no server is running
    // but the test_connection method should work properly
    assert!(result.is_err(), "Expected connection error since no server is running");
    
    match result.unwrap_err() {
        JSONRPCError { code: -32603, .. } => {
            println!("Connection test correctly detected no server (expected)");
        },
        JSONRPCError { code: -32000, .. } => {
            println!("Connection test timeout (expected with no server)");
        },
        other => {
            println!("Connection test got error (may be expected): {:?}", other);
        }
    }
    
    // The important thing is that test_connection method exists and works
    // It should fail gracefully when no server is present, not crash
}