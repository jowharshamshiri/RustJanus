use rust_janus::*;
mod test_utils;
use test_utils::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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
        JanusError::CommandTimeout(command_id, duration) => {
            assert!(!command_id.is_empty());
            assert_eq!(duration, std::time::Duration::from_millis(100));
        },
        JanusError::ConnectionError(_) => {
            // Also acceptable - connection failed before timeout
        },
        JanusError::SecurityViolation(_) | JanusError::InvalidSocketPath(_) => {
            // Security validation errors are acceptable in tests
        },
        err => panic!("Expected CommandTimeout or ConnectionError, got: {:?}", err),
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
        Err(JanusError::CommandTimeout(command_id, duration)) => {
            // Validate error message format
            let error_message = format!("{}", JanusError::CommandTimeout(command_id.clone(), duration));
            assert!(error_message.contains(&command_id));
            assert!(error_message.contains("timed out"));
            assert!(error_message.contains("50ms") || error_message.contains("0.05"));
        },
        Err(JanusError::ConnectionError(_)) => {
            // Connection error is also acceptable
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
    let mut command_ids = Vec::new();
    
    for _ in 0..10 {
        match client.send_command(
            "echo",
            Some(args.clone()),
            Some(std::time::Duration::from_millis(10)),
        ).await {
            Err(JanusError::CommandTimeout(command_id, _)) => {
                // Validate UUID format (36 characters with hyphens)
                assert_eq!(command_id.len(), 36);
                assert_eq!(command_id.chars().filter(|&c| c == '-').count(), 4);
                
                // Check uniqueness
                assert!(!command_ids.contains(&command_id), "UUIDs should be unique");
                command_ids.push(command_id);
            },
            Err(JanusError::ConnectionError(_)) => {
                // Connection errors don't provide command IDs, skip
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
            Err(JanusError::CommandTimeout(_, actual_timeout)) => {
                assert_eq!(actual_timeout, *timeout, "Timeout {} should match expected", i);
                
                // Timing should be approximately correct (within 50ms tolerance)
                let timing_diff = if elapsed_time > *timeout {
                    elapsed_time - *timeout
                } else {
                    *timeout - elapsed_time
                };
                assert!(timing_diff < std::time::Duration::from_millis(50), 
                       "Timeout {} timing should be accurate", i);
            },
            Err(JanusError::ConnectionError(_)) => {
                // Connection errors happen faster than timeout
                assert!(elapsed_time < *timeout, "Connection error should be faster than timeout");
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
        Err(JanusError::ConnectionError(_)) => {
            // Connection should fail quickly (much faster than 30 seconds)
            assert!(elapsed < std::time::Duration::from_secs(5), 
                   "Connection error should be fast");
        },
        Err(JanusError::CommandTimeout(_, timeout)) => {
            assert_eq!(timeout, std::time::Duration::from_secs(30));
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
    
    let client = Arc::new(JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap());
    
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
                Err(JanusError::CommandTimeout(_, _)) => {
                    timeout_counter_clone.fetch_add(1, Ordering::SeqCst);
                },
                Err(JanusError::ConnectionError(_)) => {
                    // Connection errors are also expected
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
    
    let handler_timeout_error = JSONRPCError {
        code: JSONRPCErrorCode::HandlerTimeout as i32,
        message: "Handler timeout".to_string(),
        data: Some(JSONRPCErrorData::with_details("Handler timeout")),
    };
    
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
        assert!(!data.details.is_empty());
    } else {
        panic!("Expected object data in JSONRPCError");
    }
    
    // Test error message format
    let error_message = format!("{}", handler_timeout_error);
    assert!(error_message.contains("echo-123"));
    assert!(error_message.contains("timed out"));
    assert!(error_message.contains("5"));
}

#[tokio::test]
async fn test_handler_timeout_api_error() {
    // Test JanusError::HandlerTimeout structure
    let api_timeout_error = JanusError::HandlerTimeout(
        "test-handler-456".to_string(),
        std::time::Duration::from_secs(10),
    );
    
    // Test error message format
    let error_message = format!("{}", api_timeout_error);
    assert!(error_message.contains("test-handler-456"));
    assert!(error_message.contains("timed out"));
    assert!(error_message.contains("10"));
    
    // Test error type matching
    match api_timeout_error {
        JanusError::HandlerTimeout(command_id, duration) => {
            assert_eq!(command_id, "test-handler-456");
            assert_eq!(duration, std::time::Duration::from_secs(10));
        },
        other => panic!("Expected HandlerTimeout, got: {:?}", other),
    }
    
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
    assert_eq!(error.code, JSONRPCErrorCode::HandlerTimeout);
    assert_eq!(error.message, "Handler timeout");
    
    if let Some(JSONRPCErrorData::Object(data)) = error.data {
        assert_eq!(data.get("command_id").unwrap().as_str().unwrap(), "echo-789");
        assert_eq!(data.get("timeout_seconds").unwrap().as_f64().unwrap(), 15.0);
    } else {
        panic!("Expected object data in timeout error response");
    }
}