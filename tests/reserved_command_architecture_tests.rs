use rust_janus::*;
use rust_janus::server::janus_server::ServerConfig;
use rust_janus::specification::manifest_parser::ManifestParser;
use serde_json::json;
use std::collections::HashMap;
use tokio::time::Duration;

mod test_utils;
use test_utils::*;

/// Reserved Command Architecture Tests
/// Tests for built-in command hardcoding and Manifest validation
/// Validates that built-in commands bypass validation and are hardcoded in implementation

#[tokio::test]
async fn test_reserved_command_validation_reject_manifests() {
    // Test that Manifests defining built-in commands are rejected
    
    // Create Manifest with reserved command
    let invalid_spec = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "commands": {
                    "ping": {  // This is a reserved command
                        "description": "Custom ping command",
                        "args": {},
                        "response": {"type": "object"}
                    },
                    "valid_command": {
                        "description": "Valid custom command",
                        "args": {},
                        "response": {"type": "object"}
                    }
                }
            }
        },
        "models": {}
    });
    
    // Try to parse the spec - should fail due to reserved command
    let spec_str = serde_json::to_string(&invalid_spec).unwrap();
    let result = ManifestParser::load_and_validate_json(&spec_str);
    
    // Should fail validation due to reserved command "ping"
    assert!(result.is_err(), "Manifest with reserved command 'ping' should be rejected");
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("reserved") || error_msg.contains("built-in") || error_msg.contains("ping"),
        "Error should mention reserved/built-in commands, got: {}", error_msg
    );
}

#[tokio::test]
async fn test_reserved_command_validation_multiple_reserved() {
    // Test rejection of Manifests with multiple reserved commands
    
    let invalid_spec = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "commands": {
                    "echo": {  // Reserved command 1
                        "description": "Custom echo command",
                        "args": {},
                        "response": {"type": "object"}
                    },
                    "get_info": {  // Reserved command 2
                        "description": "Custom get_info command",
                        "args": {},
                        "response": {"type": "object"}
                    }
                }
            }
        },
        "models": {}
    });
    
    let spec_str = serde_json::to_string(&invalid_spec).unwrap();
    let result = ManifestParser::load_and_validate_json(&spec_str);
    
    assert!(result.is_err(), "Manifest with multiple reserved commands should be rejected");
    let error_msg = result.unwrap_err().to_string().to_lowercase();
    assert!(
        error_msg.contains("reserved") || error_msg.contains("built-in") || 
        error_msg.contains("echo") || error_msg.contains("get_info"),
        "Error should mention reserved commands, got: {}", error_msg
    );
}

#[tokio::test]
async fn test_builtin_command_hardcoding_all_six_commands() {
    // Test that all 6 built-in commands are hardcoded in implementation and work without Manifest
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
    
    // Test all 6 built-in commands work without any custom handlers registered
    let builtin_commands = vec!["ping", "echo", "get_info", "validate", "slow_process", "spec"];
    
    for command in builtin_commands {
        let mut args = HashMap::new();
        
        // Add appropriate args for each command
        match command {
            "echo" => { args.insert("message".to_string(), json!("test message")); }
            "validate" => { args.insert("message".to_string(), json!("{\"test\": true}")); }
            _ => {} // Other commands don't need args
        }
        
        let result = client.send_command(command, Some(args), Some(Duration::from_secs(15))).await;
        
        assert!(result.is_ok(), "Built-in command '{}' should work without custom handlers, got: {:?}", command, result);
        let response = result.unwrap();
        assert!(response.success, "Built-in command '{}' should return success, got error: {:?}", command, response.error);
        assert!(response.result.is_some(), "Built-in command '{}' should have result", command);
    }
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_builtin_command_hardcoding_without_handlers() {
    // Test that built-in commands work even when no custom handlers are registered
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
    let result = client.send_command("ping", Some(args), Some(Duration::from_secs(5))).await;
    
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
async fn test_manifest_reserved_command_detection_parser() {
    // Test that Manifest parser detects and rejects reserved commands
    
    // Test each reserved command individually
    let reserved_commands = vec!["ping", "echo", "get_info", "validate", "slow_process", "spec"];
    
    for reserved_cmd in reserved_commands {
        let invalid_spec = json!({
            "version": "1.0.0",
            "channels": {
                "test": {
                    "description": "Test channel",
                    "commands": {
                        reserved_cmd: {
                            "description": format!("Custom {} command", reserved_cmd),
                            "args": {},
                            "response": {"type": "object"}
                        }
                    }
                }
            },
            "models": {}
        });
        
        let spec_str = serde_json::to_string(&invalid_spec).unwrap();
        let result = ManifestParser::load_and_validate_json(&spec_str);
        
        assert!(result.is_err(), "Manifest defining reserved command '{}' should be rejected", reserved_cmd);
    }
}

#[tokio::test]
async fn test_manifest_reserved_command_detection_valid_spec() {
    // Test that Manifests with only valid (non-reserved) commands are accepted
    
    let valid_spec = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "commands": {
                    "custom_command": {
                        "description": "Valid custom command",
                        "args": {
                            "param": {
                                "type": "string",
                                "required": true
                            }
                        },
                        "response": {"type": "object"}
                    },
                    "another_command": {
                        "description": "Another valid custom command",
                        "args": {},
                        "response": {"type": "string"}
                    }
                }
            }
        },
        "models": {}
    });
    
    let spec_str = serde_json::to_string(&valid_spec).unwrap();
    let result = ManifestParser::load_and_validate_json(&spec_str);
    
    assert!(result.is_ok(), "Manifest with only valid commands should be accepted, got: {:?}", result);
}

#[tokio::test]
async fn test_command_architecture_enforcement_builtin_bypass_validation() {
    // Test that built-in commands bypass API validation entirely
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
    
    // Test that built-in commands work even with validation enabled
    // (they should bypass validation entirely)
    let args = HashMap::new();
    let result = client.send_command("ping", Some(args), Some(Duration::from_secs(5))).await;
    
    // Built-in commands should work regardless of validation settings
    assert!(result.is_ok(), "Built-in ping should bypass validation and work, got: {:?}", result);
    let response = result.unwrap();
    assert!(response.success, "Built-in ping should succeed even with validation enabled");
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_command_architecture_enforcement_custom_vs_builtin() {
    // Test that custom commands require validation but built-ins don't
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
    
    // Test built-in command (should work without Manifest)
    let args = HashMap::new();
    let builtin_result = client.send_command("ping", Some(args.clone()), Some(Duration::from_secs(5))).await;
    assert!(builtin_result.is_ok(), "Built-in command should work without Manifest");
    
    // Test custom command (should fail without Manifest due to validation)
    let custom_result = client.send_command("custom_command", Some(args), Some(Duration::from_secs(5))).await;
    
    // Custom command should fail due to lack of Manifest
    match custom_result {
        Ok(response) => {
            if !response.success {
                // Should have an error related to validation or method not found
                if let Some(error) = &response.error {
                    let error_msg = error.message.to_lowercase();
                    assert!(
                        error_msg.contains("method not found") || 
                        error_msg.contains("validation") ||
                        error_msg.contains("unknown command") ||
                        error_msg.contains("specification"),
                        "Custom command should fail with validation/method error, got: {}", error.message
                    );
                }
            } else {
                panic!("Custom command should not succeed without Manifest");
            }
        }
        Err(_) => {
            // Error is also acceptable - custom command failed as expected
        }
    }
    
    // Stop server
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}

#[tokio::test]
async fn test_reserved_command_architecture_comprehensive() {
    // Comprehensive test covering all Reserved Command Architecture features
    let socket_path = "/tmp/rust_janus_reserved_comprehensive_test.sock";
    let _ = std::fs::remove_file(socket_path);
    
    // 1. Test Manifest rejection with reserved commands
    let invalid_spec = json!({
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "commands": {
                    "spec": {  // Reserved command
                        "description": "Custom spec command",
                        "args": {},
                        "response": {"type": "object"}
                    }
                }
            }
        },
        "models": {}
    });
    
    let spec_str = serde_json::to_string(&invalid_spec).unwrap();
    let parse_result = ManifestParser::load_and_validate_json(&spec_str);
    assert!(parse_result.is_err(), "Manifest with 'spec' reserved command should be rejected");
    
    // 2. Test built-in command hardcoding
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
    
    // All built-in commands should work without custom handlers
    let builtin_commands = vec!["ping", "echo", "get_info", "validate", "slow_process", "spec"];
    for cmd in builtin_commands {
        let args = HashMap::new();
        let result = client.send_command(cmd, Some(args), Some(Duration::from_secs(10))).await;
        assert!(result.is_ok(), "Built-in command '{}' should be hardcoded and work", cmd);
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
    let result = validating_client.send_command("ping", Some(args), Some(Duration::from_secs(5))).await;
    assert!(result.is_ok(), "Built-in commands should bypass validation architecture");
    
    server.stop();
    let _ = std::fs::remove_file(socket_path);
}