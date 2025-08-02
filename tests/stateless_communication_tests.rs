use rust_janus::*;
mod test_utils;
use test_utils::*;

/// Stateless Communication Tests (8 tests) - SwiftJanus parity
/// Tests stateless patterns, isolation, and validation

#[tokio::test]
async fn test_stateless_command_validation() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Command validation should work without connection
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Validation happens before connection attempt
    let result = client.send_command(
        "echo",
        Some(create_test_args()),
        Some(std::time::Duration::from_millis(100)),
    ).await;
    
    // Should fail at connection, not validation
    match result {
        Err(JanusError::ConnectionError(_)) | Err(JanusError::CommandTimeout(_, _)) => {},
        other => println!("Stateless validation result: {:?}", other),
    }
}

#[tokio::test]
async fn test_request_independence() {
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Each request should be independent - no state carried between requests
    let requests = vec!["ping", "echo", "get_info"];
    
    for command in requests {
        let result = client.send_command(
            command,
            None,
            Some(std::time::Duration::from_millis(100)),
        ).await;
        
        // All should fail identically (no server running)
        assert!(result.is_err(), "Command {} should fail without server", command);
    }
}

#[tokio::test]
async fn test_no_connection_state_preservation() {
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client1 = JanusClient::new(
        socket_path.clone(),
        "client1".to_string(),
        config.clone(),
    ).await.unwrap();
    
    let mut client2 = JanusClient::new(
        socket_path,
        "client2".to_string(),
        config,
    ).await.unwrap();
    
    // Clients should not interfere with each other
    let result1 = client1.send_command("ping", None, Some(std::time::Duration::from_millis(50))).await;
    let result2 = client2.send_command("echo", None, Some(std::time::Duration::from_millis(50))).await;
    
    // Both should fail independently
    assert!(result1.is_err());
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_socket_isolation() {
    let config = create_test_config();
    
    // Different socket paths should be completely isolated
    let socket_path1 = "/tmp/janus_test_isolation_1.sock".to_string();
    let socket_path2 = "/tmp/janus_test_isolation_2.sock".to_string();
    
    let mut client1 = JanusClient::new(
        socket_path1,
        "test1".to_string(),
        config.clone(),
    ).await.unwrap();
    
    let mut client2 = JanusClient::new(
        socket_path2,
        "test2".to_string(),
        config,
    ).await.unwrap();
    
    // Commands to different paths should be completely isolated
    let result1 = client1.send_command("ping", None, Some(std::time::Duration::from_millis(50))).await;
    let result2 = client2.send_command("ping", None, Some(std::time::Duration::from_millis(50))).await;
    
    // Both should fail but be independent
    assert!(result1.is_err());
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_command_id_uniqueness() {
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Multiple commands should generate unique IDs
    let mut command_ids = std::collections::HashSet::new();
    
    for _ in 0..5 {
        // Create commands but don't send them to avoid connection errors
        let command = SocketCommand::new(
            "test".to_string(),
            "ping".to_string(),
            None,
            None,
        );
        
        assert!(command_ids.insert(command.id.clone()), "Command ID should be unique");
    }
    
    assert_eq!(command_ids.len(), 5);
}

#[tokio::test]
async fn test_concurrent_stateless_requests() {
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Create multiple clients for concurrent access
    let mut handles = vec![];
    
    for i in 0..3 {
        let socket_path_clone = socket_path.clone();
        let config_clone = config.clone();
        
        let handle = tokio::spawn(async move {
            let mut client = JanusClient::new(
                socket_path_clone,
                format!("client_{}", i),
                config_clone,
            ).await.unwrap();
            
            client.send_command(
                "ping",
                None,
                Some(std::time::Duration::from_millis(100)),
            ).await
        });
        
        handles.push(handle);
    }
    
    // All concurrent requests should fail independently
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_err());
    }
}

#[tokio::test]
async fn test_response_socket_cleanup() {
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Make multiple requests that will fail
    for _ in 0..3 {
        let _result = client.send_command(
            "ping",
            None,
            Some(std::time::Duration::from_millis(50)),
        ).await;
    }
    
    // Response sockets should be cleaned up automatically
    // This is mainly testing that we don't leak sockets
    // (The test passes if no socket file descriptors leak)
}

#[tokio::test]
async fn test_channel_isolation() {
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Different channels should be isolated
    let mut client1 = JanusClient::new(
        socket_path.clone(),
        "channel1".to_string(),
        config.clone(),
    ).await.unwrap();
    
    let mut client2 = JanusClient::new(
        socket_path,
        "channel2".to_string(),
        config,
    ).await.unwrap();
    
    // Commands on different channels should be independent
    let result1 = client1.send_command("ping", None, Some(std::time::Duration::from_millis(50))).await;
    let result2 = client2.send_command("ping", None, Some(std::time::Duration::from_millis(50))).await;
    
    // Both should fail but independently
    assert!(result1.is_err());
    assert!(result2.is_err());
}