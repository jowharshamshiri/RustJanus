use rust_janus::*;
use rust_janus::server::janus_server::ServerConfig;
use serde_json::json;
use std::collections::HashMap;
use tokio::time::{timeout, Duration};

mod test_utils;
use test_utils::*;

/// Built-in Request Tests
/// Tests for all built-in requests: ping, echo, get_info, validate, slow_process, manifest
/// Tests actual server responses, not fake simulations

#[tokio::test]
async fn test_ping_request_with_server() {
    let socket_path = "/tmp/test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test ping request
    let args = HashMap::new();
    let result = client.send_request("ping", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify ping response
    assert!(result.is_ok(), "Ping request should succeed, got: {:?}", result);
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
async fn test_echo_request_with_server() {
    let socket_path = "/tmp/rust-janus-echo_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test echo request with message
    let mut args = HashMap::new();
    args.insert("message".to_string(), json!("Hello, Rust World!"));
    let result = client.send_request("echo", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify echo response
    assert!(result.is_ok(), "Echo request should succeed, got: {:?}", result);
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
async fn test_get_info_request_with_server() {
    let socket_path = "/tmp/rust_janus_builtin_get_info_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test get_info request
    let args = HashMap::new();
    let result = client.send_request("get_info", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify get_info response
    assert!(result.is_ok(), "Get_info request should succeed, got: {:?}", result);
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
async fn test_validate_request_valid_json() {
    let socket_path = "/tmp/rust_janus_builtin_validate_valid_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test validate request with valid JSON
    let mut args = HashMap::new();
    args.insert("message".to_string(), json!("{\"key\": \"value\", \"number\": 42}"));
    let result = client.send_request("validate", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify validate response
    assert!(result.is_ok(), "Validate request should succeed, got: {:?}", result);
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
async fn test_validate_request_invalid_json() {
    let socket_path = "/tmp/rust_janus_builtin_validate_invalid_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test validate request with invalid JSON
    let mut args = HashMap::new();
    args.insert("message".to_string(), json!("{invalid json"));
    let result = client.send_request("validate", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify validate response
    assert!(result.is_ok(), "Validate request should succeed, got: {:?}", result);
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
async fn test_slow_process_request() {
    let socket_path = "/tmp/rust_janus_builtin_slow_process_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test slow_process request with longer timeout
    let args = HashMap::new();
    let start_time = std::time::Instant::now();
    let result = client.send_request("slow_process", Some(args), Some(Duration::from_secs(10))).await;
    let elapsed = start_time.elapsed();
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify slow_process response
    assert!(result.is_ok(), "Slow_process request should succeed, got: {:?}", result);
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
async fn test_manifest_request() {
    let socket_path = "/tmp/rust_janus_builtin_manifest_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test manifest request
    let args = HashMap::new();
    let result = client.send_request("manifest", Some(args), Some(Duration::from_secs(5))).await;
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
    
    // Verify manifest response
    assert!(result.is_ok(), "Manifest request should succeed, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Manifest should return success");
    assert!(response.result.is_some(), "Manifest should have result");
    
    let result_obj = response.result.unwrap();
    assert!(result_obj.is_object(), "Manifest result should be object");
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("version"), "Manifest should contain 'version' field");
    assert!(obj.contains_key("channels"), "Manifest should contain 'channels' field");
    assert!(obj.contains_key("models"), "Manifest should contain 'models' field");
}

#[tokio::test]
async fn test_all_builtin_requests_recognized() {
    let socket_path = "/tmp/rust_janus_builtin_all_requests_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server
    let server_config = ServerConfig {
        socket_path: socket_path.to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };
    let mut server = JanusServer::new(server_config);
    server.start_listening().await.expect("Failed to start server");
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Create client with validation disabled for built-in request testing
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test that all built-in requests are recognized (don't return "method not found")
    let builtin_requests = vec!["ping", "echo", "get_info", "validate", "slow_process", "manifest"];
    
    for request in builtin_requests {
        let args = HashMap::new();
        let result = client.send_request(request, Some(args), Some(Duration::from_secs(15))).await;
        
        // All built-in requests should succeed or at least not return "method not found"
        match result {
            Ok(response) => {
                if !response.success {
                    if let Some(error) = &response.error {
                        let error_msg = error.message.to_lowercase();
                        assert!(
                            !error_msg.contains("method not found") && !error_msg.contains("unknown request"),
                            "Request '{}' should be recognized as built-in, got error: {}", request, error.message
                        );
                    }
                }
            }
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                assert!(
                    !error_msg.contains("method not found") && !error_msg.contains("unknown request"),
                    "Request '{}' should be recognized as built-in, got error: {}", request, e
                );
            }
        }
    }
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}