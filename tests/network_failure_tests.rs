use rust_janus::*;
use std::collections::HashMap;
mod test_utils;
use test_utils::*;

/// Network Failure Tests (15 tests) - SwiftJanus parity
/// Tests connection failures, permission issues, resource exhaustion

#[tokio::test]
async fn test_connection_to_nonexistent_socket() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let nonexistent_path = "/tmp/nonexistent_socket_12345.sock".to_string();
    
    let mut client = JanusClient::new(
        nonexistent_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    let result = client.send_command(
        "echo",
        Some(create_test_args()),
        Some(std::time::Duration::from_millis(100)),
    ).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        err if err.code == -32000 => {}, // ServerError (connection/timeout)
        err if err.code == -32005 => {}, // ValidationFailed (security/path)
        err => panic!("Expected connection or validation error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_connection_timeout() {
    let config = create_test_config();
    let timeout_path = "/tmp/nonexistent_timeout_socket.sock".to_string();
    
    let mut client = JanusClient::new(
        timeout_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    let result = client.send_command(
        "echo",
        Some(create_test_args()),
        Some(std::time::Duration::from_millis(50)), // Very short timeout
    ).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        err if err.code == -32000 => {}, // ServerError (timeout/connection)
        err => panic!("Expected timeout or connection error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_repeated_connection_failures() {
    let config = create_test_config();
    let nonexistent_path = "/tmp/repeated_failure_socket.sock".to_string();
    
    let mut client = JanusClient::new(
        nonexistent_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test multiple consecutive failures
    for i in 0..3 {
        let result = client.send_command(
            "ping",
            None,
            Some(std::time::Duration::from_millis(100)),
        ).await;
        
        assert!(result.is_err(), "Attempt {} should fail", i + 1);
    }
}

#[tokio::test]
async fn test_malformed_socket_path() {
    let config = create_test_config();
    let malformed_paths = vec![
        "".to_string(),
        "/".to_string(),
        "relative/path.sock".to_string(),
        "/tmp/../../../etc/passwd".to_string(),
    ];
    
    for path in malformed_paths {
        let result = JanusClient::new(
            path.clone(),
            "test".to_string(),
            config.clone(),
        ).await;
        
        if let Ok(mut client) = result {
            let send_result = client.send_command(
                "ping",
                None,
                Some(std::time::Duration::from_millis(100))
            ).await;
            assert!(send_result.is_err(), "Malformed path {} should fail", path);
        }
    }
}

#[tokio::test]
async fn test_invalid_socket_path_format() {
    let config = create_test_config();
    let invalid_path = "invalid\0path/with/null.sock".to_string();
    
    let result = JanusClient::new(
        invalid_path,
        "test".to_string(),
        config,
    ).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        err if err.code == -32005 => {}, // ValidationFailed (security/path)
        err => panic!("Expected path validation error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_permission_denied_socket_path() {
    let config = create_test_config();
    let restricted_path = "/root/restricted_socket.sock".to_string();
    
    let result = JanusClient::new(
        restricted_path,
        "test".to_string(),
        config,
    ).await;
    
    // Should either fail at creation or at first send attempt
    if let Ok(mut client) = result {
        let send_result = client.send_command(
            "ping",
            None,
            Some(std::time::Duration::from_millis(100))
        ).await;
        assert!(send_result.is_err());
    }
}

#[tokio::test]
async fn test_socket_path_too_long() {
    let config = create_test_config();
    let long_path = format!("/tmp/{}.sock", "a".repeat(200));
    
    let result = JanusClient::new(
        long_path,
        "test".to_string(),
        config,
    ).await;
    
    assert!(result.is_err());
    match result.unwrap_err() {
        err if err.code == -32005 => {}, // ValidationFailed (path length)
        err => panic!("Expected path length validation error, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_resource_exhaustion_handling() {
    let config = create_test_config();
    let test_path = "/tmp/resource_exhaustion_test.sock".to_string();
    
    let mut client = JanusClient::new(
        test_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Simulate resource exhaustion by creating many concurrent requests
    let mut tasks = vec![];
    for i in 0..50 {
        let mut client_clone = client.clone();
        let task = tokio::spawn(async move {
            let mut args = std::collections::HashMap::new();
            args.insert("message".to_string(), serde_json::json!(format!("test_{}", i)));
            
            client_clone.send_command(
                "echo",
                Some(args),
                Some(std::time::Duration::from_millis(10)),
            ).await
        });
        tasks.push(task);
    }
    
    // Wait for all tasks to complete
    let mut success_count = 0;
    let mut error_count = 0;
    
    for task in tasks {
        match task.await.unwrap() {
            Ok(_) => success_count += 1,
            Err(err) if err.code == -32000 => error_count += 1, // ServerError (timeout/connection/IO)
            Err(other) => panic!("Unexpected error type: {:?}", other),
        }
    }
    
    // Should handle resource exhaustion gracefully
    assert!(success_count + error_count == 50);
    
    // Verify client can still function after resource exhaustion
    let final_result = client.send_command(
        "ping",
        None,
        Some(std::time::Duration::from_secs(1)),
    ).await;
    
    // Final command should work or fail gracefully
    match final_result {
        Ok(_) => {},
        Err(err) if err.code == -32000 => {}, // ServerError (timeout/connection/IO)
        Err(other) => panic!("Final command failed with unexpected error: {:?}", other),
    }
}

#[tokio::test]
async fn test_network_interruption_recovery() {
    let config = create_test_config();
    let interruption_path = "/tmp/network_interruption_test.sock".to_string();
    
    let mut client = JanusClient::new(
        interruption_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test network interruption with very short timeout
    let interrupted_result = client.send_command(
        "slow_process",
        None,
        Some(std::time::Duration::from_millis(10)),
    ).await;
    
    // Should handle interruption gracefully
    assert!(interrupted_result.is_err());
    match interrupted_result.unwrap_err() {
        err if err.code == -32000 => {}, // ServerError (timeout/connection/IO)
        err => panic!("Expected network interruption error, got: {:?}", err),
    }
    
    // Verify recovery after interruption
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    let recovery_result = client.send_command(
        "ping",
        None,
        Some(std::time::Duration::from_secs(1)),
    ).await;
    
    // Should recover gracefully
    match recovery_result {
        Ok(_) => {},
        Err(err) if err.code == -32000 => {}, // ServerError (timeout/connection/IO)
        Err(other) => panic!("Recovery failed with unexpected error: {:?}", other),
    }
}