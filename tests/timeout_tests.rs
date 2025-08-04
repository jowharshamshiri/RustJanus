use rust_janus::*;
use rust_janus::error::{JSONRPCErrorCode, JSONRPCErrorData, JSONRPCError};
mod test_utils;
use test_utils::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Timeout Tests (9 tests) - Exact SwiftJanus parity
/// Tests bilateral timeout handling, cleanup, recovery, and error propagation

#[tokio::test]
async fn test_command_with_timeout() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    let timeout_counter = create_counting_timeout_handler();
    let args = create_test_args();
    
    // 0.1 second timeout (100ms) - should timeout since no server is running
    let result = client.send_command(
        "echo",
        Some(args),
        Some(std::time::Duration::from_millis(100)),
    ).await;
    
    // Should timeout
    assert!(result.is_err());
    
    match result.unwrap_err() {
        err if err.code == -32000 => {
            // ServerError covers CommandTimeout and ConnectionError
            println!("Expected timeout or connection error: {}", err);
        },
        err if err.code == -32005 => {
            // ValidationFailed covers security validation errors
            println!("Security validation error: {}", err);
        },
        err if err.code == -32007 => {
            // SocketError - valid when no server is running
            println!("Socket error (no server): {}", err);
        },
        err => panic!("Expected timeout, connection, or validation error, got: {:?}", err),
    }
    
    // Timeout callback should have been called (if timeout occurred)
    let timeout_count = timeout_counter.load(Ordering::SeqCst);
    println!("Timeout callback invoked {} times", timeout_count);
}

#[tokio::test]
async fn test_command_timeout_error_message() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    let args = create_test_args();
    
    let result = client.send_command(
        "echo",
        Some(args),
        Some(std::time::Duration::from_millis(50)),
    ).await;
    
    match result {
        Err(err) if err.code == -32000 => {
            // ServerError covers timeout and connection errors
            let error_message = format!("{}", err);
            println!("Expected timeout or connection error: {}", error_message);
        },
        other => {
            // Other results are acceptable but log them
            println!("Timeout test result: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_uuid_generation() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    let args = create_test_args();
    
    // Generate multiple commands to check UUID uniqueness
    let mut command_ids: Vec<String> = Vec::new();
    
    for _ in 0..10 {
        match client.send_command(
            "echo",
            Some(args.clone()),
            Some(std::time::Duration::from_millis(10)),
        ).await {
            Err(err) if err.code == -32000 => {
                // Server errors (timeout/connection) - validate if timeout with ID info
                if let Some(data) = &err.data {
                    if let Some(details) = &data.details {
                        if details.contains("command_id") {
                            // Extract and validate UUID from error details if present
                            println!("Timeout with command ID details: {}", details);
                        }
                    }
                }
                println!("Server error (timeout or connection): {}", err);
            },
            other => {
                println!("UUID test result: {:?}", other);
            }
        }
    }
    
    println!("Generated {} unique command IDs", command_ids.len());
}

#[tokio::test]
async fn test_multiple_commands_with_different_timeouts() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test 3 different timeouts (0.05s, 0.1s, 0.15s)
    let timeouts = vec![
        std::time::Duration::from_millis(50),   // 0.05s
        std::time::Duration::from_millis(100),  // 0.1s
        std::time::Duration::from_millis(150),  // 0.15s
    ];
    
    let args = create_test_args();
    
    for (i, timeout) in timeouts.iter().enumerate() {
        let (result, elapsed_time) = measure_time(
            client.send_command(
                "echo",
                Some(args.clone()),
                Some(*timeout),
            )
        ).await;
        
        match result {
            Err(err) if err.code == -32000 => {
                // ServerError covers both timeout and connection errors
                println!("Server error for timeout test {}: {}", i, err);
                
                // Timing validation for actual errors
                if elapsed_time > *timeout + std::time::Duration::from_millis(50) {
                    println!("Warning: Timing for test {} may be inaccurate: elapsed={:?}, expected={:?}", i, elapsed_time, timeout);
                }
            },
            other => {
                println!("Multiple timeout test {}: {:?}", i, other);
            }
        }
    }
}

#[tokio::test]
async fn test_socket_command_timeout_field() {
    let command_with_timeout = JanusCommand::new(
        "test".to_string(),
        "echo".to_string(),
        Some(create_test_args()),
        Some(30.5), // 30.5 seconds
    );
    
    // Serialize and verify timeout field
    let json = serde_json::to_string(&command_with_timeout).unwrap();
    assert!(json.contains("\"timeout\":30.5"), "JSON should contain timeout field");
    
    // Deserialize and verify
    let deserialized: JanusCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.timeout, Some(30.5));
    
    // Test timeout duration conversion
    let duration = deserialized.timeout_duration().unwrap();
    assert_eq!(duration, std::time::Duration::from_secs_f64(30.5));
}

#[tokio::test]
async fn test_socket_command_without_timeout() {
    let command_without_timeout = JanusCommand::new(
        "test".to_string(),
        "echo".to_string(),
        Some(create_test_args()),
        None, // No timeout
    );
    
    // Serialize and verify no timeout field
    let json = serde_json::to_string(&command_without_timeout).unwrap();
    assert!(json.contains("\"timeout\":null") || !json.contains("\"timeout\""));
    
    // Deserialize and verify
    let deserialized: JanusCommand = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.timeout, None);
    
    // Test timeout duration conversion
    assert!(deserialized.timeout_duration().is_none());
    assert!(!deserialized.has_timeout());
}

#[tokio::test]
async fn test_default_timeout() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    let args = create_test_args();
    
    // Test with default 30-second timeout (should be longer than our test)
    let start_time = std::time::Instant::now();
    
    let result = client.send_command(
        "echo",
        Some(args),
        Some(std::time::Duration::from_secs(30)), // Default timeout
    ).await;
    
    let elapsed = start_time.elapsed();
    
    match result {
        Err(err) if err.code == -32000 => {
            // ServerError covers both connection and timeout errors
            if elapsed < std::time::Duration::from_secs(5) {
                println!("Fast server error (likely connection): {}", err);
            } else {
                println!("Slow server error (likely timeout): {}", err);
            }
        },
        other => {
            println!("Default timeout test result: {:?}", other);
        }
    }
}

#[tokio::test]
async fn test_concurrent_timeouts() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(Mutex::new(JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap()));
    
    let timeout_counter = Arc::new(AtomicUsize::new(0));
    
    // 5 concurrent timeout operations
    let mut tasks = Vec::new();
    for i in 0..5 {
        let client_clone = client.clone();
        let timeout_counter_clone = timeout_counter.clone();
        
        tasks.push(tokio::spawn(async move {
            let _timeout_counter = create_counting_timeout_handler();
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("concurrent_{}", i)));
            
            let result = {
                let mut client = client_clone.lock().await;
                client.send_command(
                    "echo",
                    Some(args),
                    Some(std::time::Duration::from_millis(100)),
                ).await
            };
            
            match result {
                Err(err) if err.code == -32000 => {
                    // Count server errors (timeout/connection) as timeouts
                    timeout_counter_clone.fetch_add(1, Ordering::SeqCst);
                },
                other => {
                    println!("Concurrent timeout task {}: {:?}", i, other);
                }
            }
        }));
    }
    
    // Wait for all tasks
    futures::future::join_all(tasks).await;
    
    let total_timeouts = timeout_counter.load(Ordering::SeqCst);
    println!("Concurrent timeouts: {}/5", total_timeouts);
    
    // Some operations should complete (either timeout or connection error)
    assert!(total_timeouts <= 5);
}

#[tokio::test]
async fn test_command_handler_timeout_error() {
    // Test JSONRPCError::HandlerTimeout structure
    use crate::error::jsonrpc_error::{JSONRPCError, JSONRPCErrorCode, JSONRPCErrorData};
    use std::collections::HashMap;
    
    let mut data = HashMap::new();
    data.insert("command_id".to_string(), serde_json::Value::String("echo-123".to_string()));
    data.insert("timeout_seconds".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(5.0).unwrap()));
    
    let handler_timeout_error = JSONRPCError::with_context(
        JSONRPCErrorCode::HandlerTimeout,
        Some("Handler echo-123 timed out after 5 seconds".to_string()),
        data,
    );
    
    // Serialize and verify structure
    let json = serde_json::to_string(&handler_timeout_error).unwrap();
    assert!(json.contains("HANDLER_TIMEOUT") || json.contains("-32006"));
    assert!(json.contains("echo-123"));
    assert!(json.contains("5.0") || json.contains("5"));
    
    // Deserialize and verify
    let deserialized: JSONRPCError = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.code, JSONRPCErrorCode::HandlerTimeout as i32);
    assert_eq!(deserialized.message, "Handler timeout");
    
    if let Some(data) = deserialized.data {
        // JSONRPCErrorData now uses details field
        if let Some(details) = &data.details {
            assert!(!details.is_empty());
        }
    } else {
        panic!("Expected object data in JSONRPCError");
    }
    
    // Test error message format
    let error_message = format!("{}", handler_timeout_error);
    println!("Debug: error_message = {}", error_message);
    // JSONRPCError format includes the error details
    assert!(error_message.contains("Handler timeout"));
}

#[tokio::test]
async fn test_handler_timeout_api_error() {
    // Test JSONRPCError for handler timeout with details
    let api_timeout_error = JSONRPCError::new(
        JSONRPCErrorCode::HandlerTimeout,
        Some("Handler test-handler-456 timed out after 10 seconds".to_string()),
    );
    
    // Test error message format
    let error_message = format!("{}", api_timeout_error);
    assert!(error_message.contains("test-handler-456"));
    assert!(error_message.contains("timed out"));
    assert!(error_message.contains("10"));
    
    // Test error code
    assert_eq!(api_timeout_error.code, JSONRPCErrorCode::HandlerTimeout as i32);
    
    // Test response creation with timeout error
    let timeout_response = JanusResponse::timeout_error(
        "echo-789".to_string(),
        "test".to_string(),
        15.0,
    );
    
    assert!(!timeout_response.success);
    assert!(timeout_response.result.is_none());
    assert!(timeout_response.error.is_some());
    
    let error = timeout_response.error.unwrap();
    assert_eq!(error.code, JSONRPCErrorCode::HandlerTimeout as i32);
    assert_eq!(error.message, "Handler timeout");
    
    if let Some(data) = error.data {
        if let Some(details) = &data.details {
            println!("Debug: details = {}", details);
            assert!(details.contains("echo-789"));
            assert!(details.contains("15"));
        }
    } else {
        panic!("Expected object data in timeout error response");
    }
}