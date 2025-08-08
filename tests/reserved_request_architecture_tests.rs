use rust_janus::*;
use rust_janus::server::janus_server::ServerConfig;
use rust_janus::manifest::manifest_parser::ManifestParser;
use serde_json::json;
use std::collections::HashMap;
use tokio::time::Duration;

mod test_utils;
use test_utils::*;

/// Reserved Request Architecture Tests
/// Tests for built-in request hardcoding and Manifest validation
/// Validates that built-in requests bypass validation and are hardcoded in implementation

#[tokio::test]
async fn test_reserved_request_validation_reject_manifests() {
    // Test that Manifests defining built-in requests are rejected
    
    // Create Manifest with reserved request
    let invalid_manifest = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "requests": {
                    "ping": {  // This is a reserved request
                        "description": "Custom ping request",
                        "args": {},
                        "response": {"type": "object"}
                    },
                    "valid_request": {
                        "description": "Valid custom request",
                        "args": {},
                        "response": {"type": "object"}
                    }
                }
            }
        },
        "models": {}
    });
    
    // Try to parse the manifest - should fail due to reserved request
    let manifest_str = serde_json::to_string(&invalid_manifest).unwrap();
    let result = ManifestParser::load_and_validate_json(&manifest_str);
    
    // Should fail validation due to reserved request "ping"
    assert!(result.is_err(), "Manifest with reserved request 'ping' should be rejected");
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("reserved") || error_msg.contains("built-in") || error_msg.contains("ping"),
        "Error should mention reserved/built-in requests, got: {}", error_msg
    );
}

#[tokio::test]
async fn test_reserved_request_validation_multiple_reserved() {
    // Test rejection of Manifests with multiple reserved requests
    
    let invalid_manifest = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "requests": {
                    "echo": {  // Reserved request 1
                        "description": "Custom echo request",
                        "args": {},
                        "response": {"type": "object"}
                    },
                    "get_info": {  // Reserved request 2
                        "description": "Custom get_info request",
                        "args": {},
                        "response": {"type": "object"}
                    }
                }
            }
        },
        "models": {}
    });
    
    let manifest_str = serde_json::to_string(&invalid_manifest).unwrap();
    let result = ManifestParser::load_and_validate_json(&manifest_str);
    
    assert!(result.is_err(), "Manifest with multiple reserved requests should be rejected");
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("reserved") || error_msg.contains("built-in") || 
        error_msg.contains("echo") || error_msg.contains("get_info"),
        "Error should mention reserved requests, got: {}", error_msg
    );
}

#[tokio::test]
async fn test_builtin_request_hardcoding_all_six_requests() {
    // Test that all 6 built-in requests are hardcoded in implementation and work without Manifest
    let socket_path = "/tmp/rust_janus_reserved_builtin_hardcoding_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start server without registering ANY handlers
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
    
    // Create client with validation disabled to test pure built-in functionality
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test all 6 built-in requests work without any custom handlers registered
    let builtin_requests = vec!["ping", "echo", "get_info", "validate", "slow_process", "manifest"];
    
    for request in builtin_requests {
        let mut args = HashMap::new();
        
        // Add appropriate args for each request
        match request {
            "echo" => { args.insert("message".to_string(), json!("test message")); }
            "validate" => { args.insert("message".to_string(), json!("{\"test\": true}")); }
            _ => {} // Other requests don't need args
        }
        
        let result = client.send_request(request, Some(args), Some(Duration::from_secs(15))).await;
        
        assert!(result.is_ok(), "Built-in request '{}' should work without custom handlers, got: {:?}", request, result);
        let response = result.unwrap();
        assert!(response.success, "Built-in request '{}' should return success, got error: {:?}", request, response.error);
        assert!(response.result.is_some(), "Built-in request '{}' should have result", request);
    }
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_builtin_request_hardcoding_without_handlers() {
    // Test that built-in requests work even when no custom handlers are registered
    let socket_path = "/tmp/rust_janus_reserved_no_handlers_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // Start completely empty server
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
    
    // Create client
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test that ping works even with no registered handlers
    let args = HashMap::new();
    let result = client.send_request("ping", Some(args), Some(Duration::from_secs(5))).await;
    
    assert!(result.is_ok(), "Built-in ping should work without any registered handlers");
    let response = result.unwrap();
    assert!(response.success, "Built-in ping should succeed");
    
    let result_obj = response.result.unwrap();
    let obj = result_obj.as_object().unwrap();
    assert!(obj.contains_key("pong"), "Built-in ping should return 'pong' field");
    assert_eq!(obj.get("pong").unwrap().as_bool().unwrap(), true);
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_manifest_reserved_request_detection_parser() {
    // Test that Manifest parser detects and rejects reserved requests
    
    // Test each reserved request individually
    let reserved_requests = vec!["ping", "echo", "get_info", "validate", "slow_process", "manifest"];
    
    for reserved_cmd in reserved_requests {
        let invalid_manifest = json!({
            "version": "1.0.0",
            "channels": {
                "test": {
                    "description": "Test channel",
                    "requests": {
                        reserved_cmd: {
                            "description": format!("Custom {} request", reserved_cmd),
                            "args": {},
                            "response": {"type": "object"}
                        }
                    }
                }
            },
            "models": {}
        });
        
        let manifest_str = serde_json::to_string(&invalid_manifest).unwrap();
        let result = ManifestParser::load_and_validate_json(&manifest_str);
        
        assert!(result.is_err(), "Manifest defining reserved request '{}' should be rejected", reserved_cmd);
    }
}

#[tokio::test]
async fn test_manifest_reserved_request_detection_valid_manifest() {
    // Test that Manifests with only valid (non-reserved) requests are accepted
    
    let valid_manifest = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "requests": {
                    "custom_request": {
                        "description": "Valid custom request",
                        "args": {
                            "param": {
                                "type": "string",
                                "required": true
                            }
                        },
                        "response": {"type": "object"}
                    },
                    "another_request": {
                        "description": "Another valid custom request",
                        "args": {},
                        "response": {"type": "string"}
                    }
                }
            }
        },
        "models": {}
    });
    
    let manifest_str = serde_json::to_string(&valid_manifest).unwrap();
    let result = ManifestParser::load_and_validate_json(&manifest_str);
    
    assert!(result.is_ok(), "Manifest with only valid requests should be accepted, got: {:?}", result);
}

#[tokio::test]
async fn test_request_architecture_enforcement_builtin_bypass_validation() {
    // Test that built-in requests bypass API validation entirely
    let socket_path = "/tmp/rust_janus_reserved_bypass_validation_test.sock";
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
    
    // Create client with validation ENABLED
    let mut config = create_test_config();
    config.enable_validation = true; // Enable validation
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test that built-in requests work even with validation enabled
    // (they should bypass validation entirely)
    let args = HashMap::new();
    let result = client.send_request("ping", Some(args), Some(Duration::from_secs(5))).await;
    
    // Built-in requests should work regardless of validation settings
    assert!(result.is_ok(), "Built-in ping should bypass validation and work, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Built-in ping should succeed even with validation enabled");
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_request_architecture_enforcement_custom_vs_builtin() {
    // Test that custom requests require validation but built-ins don't
    let socket_path = "/tmp/rust_janus_reserved_custom_vs_builtin_test.sock";
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
    
    // Create client with validation enabled but no Manifest provided
    let mut config = create_test_config();
    config.enable_validation = true;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // Test built-in request (should work without Manifest)
    let args = HashMap::new();
    let builtin_result = client.send_request("ping", Some(args.clone()), Some(Duration::from_secs(5))).await;
    assert!(builtin_result.is_ok(), "Built-in request should work without Manifest");
    
    // Test custom request (should fail without Manifest due to validation)
    let custom_result = client.send_request("custom_request", Some(args), Some(Duration::from_secs(5))).await;
    
    // Custom request should fail due to lack of Manifest
    match custom_result {
        Ok(response) => {
            if !response.success {
                // Should have an error related to validation or method not found
                if let Some(error) = &response.error {
                    let error_msg = error.message.to_lowercase();
                    assert!(
                        error_msg.contains("method not found") || 
                        error_msg.contains("validation") ||
                        error_msg.contains("unknown request") ||
                        error_msg.contains("manifest"),
                        "Custom request should fail with validation/method error, got: {}", error.message
                    );
                }
            } else {
                panic!("Custom request should not succeed without Manifest");
            }
        }
        Err(_) => {
            // Error is also acceptable - custom request failed as expected
        }
    }
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_reserved_request_architecture_comprehensive() {
    // Comprehensive test covering all Reserved Request Architecture features
    let socket_path = "/tmp/rust_janus_reserved_comprehensive_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // 1. Test Manifest rejection with reserved requests
    let invalid_manifest = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "requests": {
                    "manifest": {  // Reserved request
                        "description": "Custom manifest request",
                        "args": {},
                        "response": {"type": "object"}
                    }
                }
            }
        },
        "models": {}
    });
    
    let manifest_str = serde_json::to_string(&invalid_manifest).unwrap();
    let parse_result = ManifestParser::load_and_validate_json(&manifest_str);
    assert!(parse_result.is_err(), "Manifest with 'manifest' reserved request should be rejected");
    
    // 2. Test built-in request hardcoding
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
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let mut config = create_test_config();
    config.enable_validation = false;
    let mut client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        config,
    ).await.expect("Failed to create client");
    
    // All built-in requests should work without custom handlers
    let builtin_requests = vec!["ping", "echo", "get_info", "validate", "slow_process", "manifest"];
    for cmd in builtin_requests {
        let args = HashMap::new();
        let result = client.send_request(cmd, Some(args), Some(Duration::from_secs(10))).await;
        assert!(result.is_ok(), "Built-in request '{}' should be hardcoded and work", cmd);
    }
    
    // 3. Test architecture enforcement (built-ins bypass validation)
    let mut validating_config = create_test_config();
    validating_config.enable_validation = true;
    let mut validating_client = JanusClient::new(
        socket_path.to_string(),
        "test".to_string(),
        validating_config,
    ).await.expect("Failed to create validating client");
    
    // Built-in should still work with validation enabled
    let args = HashMap::new();
    let result = validating_client.send_request("ping", Some(args), Some(Duration::from_secs(5))).await;
    assert!(result.is_ok(), "Built-in requests should bypass validation architecture");
    
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}