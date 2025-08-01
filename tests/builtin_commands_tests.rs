use rust_janus::*;
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};

mod test_utils;
use test_utils::*;

/// Built-in Command Tests
/// Tests for all built-in commands: ping, echo, get_info, validate, slow_process, spec
/// Tests actual server responses, not fake simulations

#[tokio::test]
async fn test_ping_command_with_server() {
    let socket_path = "/tmp/test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test ping command
    let args = HashMap::new();
    let result = client.send_command("ping", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify ping response
    assert!(result.is_ok(), "Ping command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Ping should return success");
    assert!(response.result.is_some(), "Ping should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Ping result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("pong"), "Ping should contain 'pong' field");
    assert_eq!(obj.get("pong").unwrap().as_bool().unwrap(), true);
    assert!(obj.contains_key("timestamp"), "Ping should contain 'timestamp' field");
}

#[tokio::test]
async fn test_echo_command_with_server() {
    let socket_path = "/tmp/rust-janus-echo_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test echo command with message
    let mut args = HashMap::new();
    args.insert("message".to_string(), json!("Hello, Rust World!"));
    let result = client.send_command("echo", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify echo response
    assert!(result.is_ok(), "Echo command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Echo should return success");
    assert!(response.result.is_some(), "Echo should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Echo result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("echo"), "Echo should contain 'echo' field");
    assert_eq!(obj.get("echo").unwrap().as_str().unwrap(), "Hello, Rust World!");
}

#[tokio::test]
async fn test_get_info_command_with_server() {
    let socket_path = "/tmp/rust_janus_builtin_get_info_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test get_info command
    let args = HashMap::new();
    let result = client.send_command("get_info", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify get_info response
    assert!(result.is_ok(), "Get_info command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Get_info should return success");
    assert!(response.result.is_some(), "Get_info should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Get_info result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("implementation"), "Get_info should contain 'implementation' field");
    assert_eq!(obj.get("implementation").unwrap().as_str().unwrap(), "Rust");
    assert!(obj.contains_key("version"), "Get_info should contain 'version' field");
    assert!(obj.contains_key("protocol"), "Get_info should contain 'protocol' field");
    assert_eq!(obj.get("protocol").unwrap().as_str().unwrap(), "SOCK_DGRAM");
}

#[tokio::test]
async fn test_validate_command_valid_json() {
    let socket_path = "/tmp/rust_janus_builtin_validate_valid_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test validate command with valid JSON
    let mut args = HashMap::new();
    args.insert("message".to_string(), json!("{\"key\": \"value\", \"number\": 42}"));
    let result = client.send_command("validate", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify validate response
    assert!(result.is_ok(), "Validate command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Validate should return success");
    assert!(response.result.is_some(), "Validate should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Validate result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("valid"), "Validate should contain 'valid' field");
    assert_eq!(obj.get("valid").unwrap().as_bool().unwrap(), true);
    assert!(obj.contains_key("data"), "Validate should contain 'data' field with parsed JSON");
}

#[tokio::test]
async fn test_validate_command_invalid_json() {
    let socket_path = "/tmp/rust_janus_builtin_validate_invalid_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test validate command with invalid JSON
    let mut args = HashMap::new();
    args.insert("message".to_string(), json!("{invalid json"));
    let result = client.send_command("validate", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify validate response
    assert!(result.is_ok(), "Validate command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Validate should return success");
    assert!(response.result.is_some(), "Validate should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Validate result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("valid"), "Validate should contain 'valid' field");
    assert_eq!(obj.get("valid").unwrap().as_bool().unwrap(), false);
    assert!(obj.contains_key("error"), "Validate should contain 'error' field");
    assert!(obj.contains_key("reason"), "Validate should contain 'reason' field");
}

#[tokio::test]
async fn test_slow_process_command() {
    let socket_path = "/tmp/rust_janus_builtin_slow_process_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test slow_process command with longer timeout
    let args = HashMap::new();
    let start_time = std::time::Instant::now();
    let result = client.send_command("slow_process", Some(args), Some(Duration::from_secs(10))).await;
    let elapsed = start_time.elapsed();
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify slow_process response
    assert!(result.is_ok(), "Slow_process command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Slow_process should return success");
    assert!(response.result.is_some(), "Slow_process should have result");
    
    // Should have taken at least 2 seconds
    assert!(elapsed >= Duration::from_secs(2), "Slow_process should take at least 2 seconds, took: {:?}", elapsed);
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Slow_process result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("processed"), "Slow_process should contain 'processed' field");
    assert_eq!(obj.get("processed").unwrap().as_bool().unwrap(), true);
    assert!(obj.contains_key("delay"), "Slow_process should contain 'delay' field");
    assert_eq!(obj.get("delay").unwrap().as_str().unwrap(), "2000ms");
}

#[tokio::test]
async fn test_spec_command() {
    let socket_path = "/tmp/rust_janus_builtin_spec_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test spec command
    let args = HashMap::new();
    let result = client.send_command("spec", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify spec response
    assert!(result.is_ok(), "Spec command should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Spec should return success");
    assert!(response.result.is_some(), "Spec should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Spec result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("version"), "Spec should contain 'version' field");
    assert!(obj.contains_key("channels"), "Spec should contain 'channels' field");
    assert!(obj.contains_key("models"), "Spec should contain 'models' field");
}

#[tokio::test]
async fn test_all_builtin_commands_recognized() {
    let socket_path = "/tmp/rust_janus_builtin_all_commands_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let mut server = JanusServer::new();
    server.start_listening(socket_path).await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in command testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test that all built-in commands are recognized (don't return "method not found")
    let builtin_commands = vec!["ping", "echo", "get_info", "validate", "slow_process", "spec"];
    
    for command in builtin_commands {
        let args = HashMap::new();
        let result = client.send_command(command, Some(args), Some(Duration::from_secs(15))).await;
        
        // All built-in commands should succeed or at least not return "method not found"
        match result {
            Ok(response) => {
                if !response.success {
                    if let Some(error) = &response.error {
                        let error_msg = error.message.to_lowercase();
                        assert!(
                            !error_msg.contains("method not found") && !error_msg.contains("unknown command"),
                            "Command '{}' should be recognized as built-in, got error: {}", command, error.message
                        );
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    !error_msg.contains("method not found") && !error_msg.contains("unknown command"),
                    "Command '{}' should be recognized as built-in, got error: {}", command, e
                );
            }
        }
    }
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}