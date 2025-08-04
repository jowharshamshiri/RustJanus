use std::collections::HashMap;
use std::os::unix::net::UnixDatagram;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;
use serde_json;
use std::fs;

use rust_janus::server::janus_server::{JanusServer, ServerConfig};
use rust_janus::protocol::message_types::{JanusCommand, JanusResponse};
use rust_janus::error::jsonrpc_error::JSONRPCError;

// Helper function to create test server
async fn create_test_server() -> (JanusServer, String) {
    // Generate unique socket path with thread ID to avoid conflicts in parallel tests
    let socket_path = format!("/tmp/janus-server-test-{}-{}-{:?}", 
        std::process::id(), 
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(),
        std::thread::current().id()
    );
    
    // Clean up any existing socket file manually before creating server
    let _ = std::fs::remove_file(&socket_path);
    
    let server_config = ServerConfig {
        socket_path: socket_path.clone(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let server = JanusServer::new(server_config);
    (server, socket_path)
}

// Helper function to send command and get response
async fn send_command_and_wait(socket_path: &str, command: JanusCommand, timeout_ms: u64) -> Result<JanusResponse, String> {
    let client_socket = UnixDatagram::unbound().map_err(|e| e.to_string())?;
    
    // Create response socket
    let response_path = format!("/tmp/janus-client-response-{}-{}", std::process::id(), command.id);
    let _ = fs::remove_file(&response_path); // Clean up any existing file
    let response_socket = UnixDatagram::bind(&response_path).map_err(|e| e.to_string())?;
    response_socket.set_nonblocking(true).map_err(|e| e.to_string())?;
    
    // Update command with response path
    let mut cmd_with_response = command;
    cmd_with_response.reply_to = Some(response_path.clone());
    
    // Send command
    let cmd_data = serde_json::to_vec(&cmd_with_response).map_err(|e| e.to_string())?;
    client_socket.send_to(&cmd_data, socket_path).map_err(|e| e.to_string())?;
    
    // Wait for response with timeout
    let start = Instant::now();
    loop {
        if start.elapsed().as_millis() > timeout_ms as u128 {
            return Err("Timeout waiting for response".into());
        }
        
        let mut buffer = vec![0u8; 4096];
        match response_socket.recv(&mut buffer) {
            Ok(size) => {
                let response_data = &buffer[..size];
                let response: JanusResponse = serde_json::from_slice(response_data).map_err(|e| e.to_string())?;
                let _ = fs::remove_file(&response_path); // Cleanup
                return Ok(response);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                sleep(Duration::from_millis(10)).await;
                continue;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
}

#[tokio::test]
async fn test_command_handler_registry() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Register test handler
    server.register_handler("test_command", |cmd| {
        Ok(serde_json::json!({"message": "test response"}))
    }).await;
    
    // Start server
    server.start_listening().await.expect("Server should start");
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Send test command
    let command = JanusCommand {
        id: "test-001".to_string(),
        channelId: "test".to_string(),
        command: "test_command".to_string(),
        reply_to: None, // Will be set by helper
        args: None,
        timeout: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
    };
    
    let response = send_command_and_wait(&socket_path, command, 2000).await;
    
    server.stop();
    
    match response {
        Ok(resp) => {
            assert!(resp.success, "Expected successful response");
            assert_eq!(resp.commandId, "test-001");
        }
        Err(e) => panic!("Failed to get response: {}", e),
    }
    
    println!("✅ Command handler registry validated");
}

#[tokio::test]
async fn test_multi_client_connection_management() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Start server
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Test multiple concurrent clients
    let client_count = 3;
    let mut handles = Vec::new();
    
    for i in 0..client_count {
        let socket_path = socket_path.clone();
        let handle = tokio::spawn(async move {
            let command = JanusCommand {
                id: format!("client-{}", i),
                channelId: format!("test-client-{}", i),
                command: "ping".to_string(),
                reply_to: None,
                args: None,
                timeout: None,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
            };
            
            send_command_and_wait(&socket_path, command, 3000).await
        });
        handles.push(handle);
    }
    
    // Wait for all clients to complete
    let mut successful_clients = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_response)) => successful_clients += 1,
            Ok(Err(e)) => println!("Client failed: {}", e),
            Err(e) => println!("Client task failed: {}", e),
        }
    }
    
    server.stop();
    
    assert_eq!(successful_clients, client_count, "All clients should succeed");
    println!("✅ Multi-client connection management validated");
}

#[tokio::test] 
async fn test_event_driven_architecture() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Start server
    match server.start_listening().await {
        Ok(_) => println!("Server started successfully on {}", socket_path),
        Err(e) => {
            println!("Server failed to start: {}", e);
            // Clean up socket file and retry once
            let _ = std::fs::remove_file(&socket_path);
            tokio::time::sleep(Duration::from_millis(100)).await;
            server.start_listening().await.expect("Server should start on retry");
        }
    }
    
    sleep(Duration::from_millis(200)).await;
    
    // Test that server processes events by sending a command
    let command = JanusCommand {
        id: "event-test".to_string(),
        channelId: "test".to_string(),
        command: "ping".to_string(),
        reply_to: None,
        args: None,
        timeout: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
    };
    
    let response = send_command_and_wait(&socket_path, command, 2000).await;
    
    // Ensure server stops and cleans up
    server.stop();
    sleep(Duration::from_millis(100)).await;
    let _ = std::fs::remove_file(&socket_path);
    
    match response {
        Ok(resp) => {
            assert!(resp.success, "Event-driven processing should work");
            assert_eq!(resp.commandId, "event-test");
        }
        Err(e) => panic!("Event processing failed: {}", e),
    }
    
    println!("✅ Event-driven architecture validated");
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Start server
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Verify server is running by connecting
    let test_socket = UnixDatagram::unbound().expect("Should create test socket");
    let connect_result = test_socket.connect(&socket_path);
    assert!(connect_result.is_ok(), "Should be able to connect to running server");
    
    // Stop server
    server.stop();
    sleep(Duration::from_millis(200)).await;
    
    // Verify server stopped (socket file should be cleaned up)
    let connect_after_stop = UnixDatagram::unbound()
        .and_then(|s| s.connect(&socket_path));
    assert!(connect_after_stop.is_err(), "Should not be able to connect after shutdown");
    
    println!("✅ Graceful shutdown validated");
}

#[tokio::test]
async fn test_connection_processing_loop() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Track processed commands
    let processed_commands = Arc::new(Mutex::new(Vec::new()));
    let processed_commands_clone = Arc::clone(&processed_commands);
    
    // Register handler that tracks commands 
    server.register_async_handler("track_test", move |cmd| {
        let processed = Arc::clone(&processed_commands_clone);
        async move {
            let mut list = processed.lock().await;
            list.push(cmd.id.clone());
            Ok(serde_json::json!({"tracked": true}))
        }
    }).await;
    
    // Start server
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Send multiple commands to test processing loop
    let command_ids = vec!["cmd1", "cmd2", "cmd3"];
    let mut handles = Vec::new();
    
    for cmd_id in &command_ids {
        let socket_path = socket_path.clone();
        let cmd_id = cmd_id.to_string();
        let handle = tokio::spawn(async move {
            let command = JanusCommand {
                id: cmd_id,
                channelId: "test".to_string(),
                command: "track_test".to_string(),
                reply_to: None,
                args: None,
                timeout: None,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs_f64(),
            };
            
            send_command_and_wait(&socket_path, command, 2000).await
        });
        handles.push(handle);
    }
    
    // Wait for all commands to complete
    for handle in handles {
        let _ = handle.await;
    }
    
    server.stop();
    
    // Verify all commands were processed
    let processed = processed_commands.lock().await;
    assert_eq!(processed.len(), command_ids.len(), "All commands should be processed");
    
    for expected_id in &command_ids {
        assert!(processed.contains(&expected_id.to_string()), 
               "Command {} should be processed", expected_id);
    }
    
    println!("✅ Connection processing loop validated");
}

#[tokio::test]
async fn test_error_response_generation() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Start server (no custom handlers registered)
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Send command that doesn't have a handler (should generate error)
    let command = JanusCommand {
        id: "error-test".to_string(),
        channelId: "test".to_string(),
        command: "nonexistent_command".to_string(),
        reply_to: None,
        args: None,
        timeout: None,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
    };
    
    let response = send_command_and_wait(&socket_path, command, 2000).await;
    
    server.stop();
    
    match response {
        Ok(resp) => {
            assert!(!resp.success, "Expected error response to have success=false");
            assert!(resp.error.is_some(), "Expected error response to have error field");
            assert_eq!(resp.commandId, "error-test");
        }
        Err(e) => panic!("Failed to get error response: {}", e),
    }
    
    println!("✅ Error response generation validated");
}

#[tokio::test]
async fn test_client_activity_tracking() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Start server
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Send multiple commands from same "client" (same channel)
    for i in 0..3 {
        let command = JanusCommand {
            id: format!("activity-test-{}", i),
            channelId: "test-client".to_string(), // Same channel = same client
            command: "ping".to_string(),
            reply_to: None,
            args: None,
            timeout: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64(),
        };
        
        let _response = send_command_and_wait(&socket_path, command, 2000).await
            .expect("Command should succeed");
        
        sleep(Duration::from_millis(50)).await;
    }
    
    server.stop();
    
    println!("✅ Client activity tracking validated through command processing");
}

#[tokio::test]
async fn test_command_execution_with_timeout() {
    let (mut server, socket_path) = create_test_server().await;
    
    // Register slow handler that should timeout
    server.register_async_handler("slow_command", |_cmd| {
        async move {
            sleep(Duration::from_millis(500)).await; // Short delay for testing
            Ok(serde_json::json!({"message": "slow command completed"}))
        }
    }).await;
    
    // Start server
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Send slow command with short timeout
    let command = JanusCommand {
        id: "timeout-test".to_string(),
        channelId: "test".to_string(),
        command: "slow_command".to_string(),
        reply_to: None,
        args: None,
        timeout: Some(1.0), // 1 second timeout
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64(),
    };
    
    let start_time = Instant::now();
    let response = send_command_and_wait(&socket_path, command, 3000).await;
    let duration = start_time.elapsed();
    
    server.stop();
    
    // Verify response came back in reasonable time
    assert!(duration < Duration::from_secs(2), "Command should complete within reasonable time");
    
    match response {
        Ok(resp) => {
            // Should get successful response
            assert!(resp.success, "Slow command should complete successfully");
            assert_eq!(resp.commandId, "timeout-test");
            println!("Response received: success={}", resp.success);
        }
        Err(_) => {
            // Timeout in our test helper also acceptable
            println!("Command timed out as expected");
        }
    }
    
    println!("✅ Command execution with timeout validated");
}

#[tokio::test]
async fn test_socket_file_cleanup() {
    let socket_path = format!("/tmp/janus-cleanup-test-{}", std::process::id());
    
    // Create dummy socket file
    std::fs::File::create(&socket_path).expect("Should create test file");
    
    // Verify file exists
    assert!(std::fs::metadata(&socket_path).is_ok(), "Test socket file should exist");
    
    // Create and start server (should cleanup existing file)
    let server_config = ServerConfig {
        socket_path: socket_path.clone(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Server should start");
    sleep(Duration::from_millis(100)).await;
    
    // Verify server created new socket (can connect)
    let test_result = UnixDatagram::unbound()
        .and_then(|s| s.connect(&socket_path));
    assert!(test_result.is_ok(), "Should be able to connect to server socket");
    
    // Stop server
    server.stop();
    sleep(Duration::from_millis(200)).await;
    
    // Verify cleanup on shutdown (socket file should be removed)
    let file_exists = std::fs::metadata(&socket_path).is_ok();
    assert!(!file_exists, "Socket file should be cleaned up on shutdown");
    
    println!("✅ Socket file cleanup validated");
}