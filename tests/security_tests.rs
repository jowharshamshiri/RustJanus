use rust_unix_sock_api::*;

mod test_utils;
use test_utils::*;
use std::collections::HashMap;

/// Security Tests (15 tests) - Exact SwiftUnixSockAPI parity
/// Tests path traversal, input injection, protocol security, and resource limits

#[tokio::test]
async fn test_path_traversal_attack() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    
    let malicious_paths = get_malicious_socket_paths();
    
    for malicious_path in malicious_paths {
        let result = UnixSockApiDatagramClient::new(
            malicious_path.clone(),
            "test-channel".to_string(),
            Some(api_spec.clone()),
            config.clone(),
        );
        
        assert!(result.is_err(), "Should reject malicious path: {}", malicious_path);
        
        match result.unwrap_err() {
            UnixSockApiError::SecurityViolation(msg) => {
                assert!(msg.contains("path traversal") || msg.contains("Security violation"));
            },
            UnixSockApiError::InvalidSocketPath(_) => {
                // Also acceptable for path validation
            },
            err => panic!("Expected SecurityViolation or InvalidSocketPath, got: {:?}", err),
        }
    }
}

#[tokio::test]
async fn test_invalid_socket_path_characters() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    
    let invalid_paths = vec![
        "/tmp/socket\0injection.sock",  // Null byte injection
        "/tmp/socket\r\ninjection.sock", // CRLF injection
        "/tmp/socket\tinjection.sock",   // Tab injection
    ];
    
    for invalid_path in invalid_paths {
        let result = UnixSockApiDatagramClient::new(
            invalid_path.to_string(),
            "test-channel".to_string(),
            Some(api_spec.clone()),
            config.clone(),
        );
        
        assert!(result.is_err(), "Should reject path with null bytes: {}", invalid_path);
    }
}

#[tokio::test]
async fn test_socket_path_length_limits() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    
    // Create a path longer than 108 characters (Unix socket limit)
    let long_path = format!("/tmp/{}.sock", "x".repeat(200));
    
    let result = UnixSockApiDatagramClient::new(
        long_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    );
    
    assert!(result.is_err());
    match result.unwrap_err() {
        UnixSockApiError::InvalidSocketPath(msg) => {
            assert!(msg.contains("too long") || msg.contains("108 character"));
        },
        err => panic!("Expected InvalidSocketPath, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_channel_id_injection_attacks() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let malicious_channel_ids = get_malicious_channel_ids();
    
    for malicious_id in malicious_channel_ids {
        let result = UnixSockApiDatagramClient::new(
            socket_path.clone(),
            malicious_id.clone(),
            Some(api_spec.clone()),
            config.clone(),
        );
        
        assert!(result.is_err(), "Should reject malicious channel ID: {}", malicious_id);
        
        match result.unwrap_err() {
            UnixSockApiError::InvalidChannel(_) | UnixSockApiError::SecurityViolation(_) => {
                // Expected
            },
            err => panic!("Expected InvalidChannel or SecurityViolation, got: {:?}", err),
        }
    }
}

#[tokio::test]
async fn test_command_injection_in_arguments() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap();
    
    let malicious_arguments = vec![
        "'; rm -rf /; --",           // SQL + command injection
        "$(cat /etc/passwd)",        // Command substitution
        "`rm -rf /`",               // Backtick command execution
        "arg && rm -rf /",          // Command chaining
        "arg | cat /etc/shadow",    // Pipe injection
        "arg; shutdown -h now",     // Command termination
        "../../../etc/passwd",      // Path traversal in args
        "arg\0injection",           // Null byte injection
        "arg\r\ninjection",         // CRLF injection
    ];
    
    for malicious_arg in malicious_arguments {
        let mut args = HashMap::new();
        args.insert("test_arg".to_string(), serde_json::Value::String(malicious_arg.to_string()));
        
        // This should not crash or execute commands, just validate
        let result = client.send_command(
            "test-command",
            Some(args),
            Some(std::time::Duration::from_millis(100)),
        ).await;
        
        // The command should either succeed (if server validates) or fail gracefully
        // It should never execute the malicious command
        match result {
            Ok(_) => {}, // Server validated and processed safely
            Err(UnixSockApiError::ConnectionError(_)) => {}, // Expected - no server running
            Err(UnixSockApiError::CommandTimeout(_, _)) => {}, // Expected - timeout
            Err(err) => {
                // Ensure it's not a security breach
                assert!(!format!("{:?}", err).contains("command executed"));
            }
        }
    }
}

#[tokio::test]
async fn test_malformed_json_attacks() {
    // Test malformed JSON patterns that could cause parsing vulnerabilities
    let malformed_patterns = get_malformed_json_patterns();
    
    for pattern in malformed_patterns {
        let result = serde_json::from_str::<SocketCommand>(pattern);
        assert!(result.is_err(), "Should reject malformed JSON: {}", pattern);
    }
}

#[tokio::test]
async fn test_unicode_normalization_attacks() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let unicode_attacks = vec![
        "normal_channel",                    // Normal case
        "café",                             // Unicode with accents
        "🚀channel",                        // Emoji prefix
        "channel\u{200B}hidden",            // Zero-width space
        "channel\u{FEFF}bom",               // Byte order mark
        "channel\u{202E}reverse",           // Right-to-left override
        "channel\u{2066}isolate",           // Directional isolate
    ];
    
    for unicode_channel in unicode_attacks {
        let result = UnixSockApiDatagramClient::new(
            socket_path.clone(),
            unicode_channel.to_string(),
            Some(api_spec.clone()),
            config.clone(),
        );
        
        // Should either accept valid Unicode or reject with proper error
        match result {
            Ok(_) => {
                // Valid Unicode characters should be accepted
                assert!(unicode_channel.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'));
            },
            Err(UnixSockApiError::InvalidChannel(_)) => {
                // Invalid characters should be rejected
            },
            Err(err) => panic!("Unexpected error for Unicode channel '{}': {:?}", unicode_channel, err),
        }
    }
}

#[tokio::test]
async fn test_large_payload_attacks() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap();
    
    let payload_sizes = vec![
        1_000,      // 1KB - should be fine
        100_000,    // 100KB - should be fine
        1_000_000,  // 1MB - should be fine with default config
        5_000_000,  // 5MB - should be fine (within args limit)
        10_000_000, // 10MB - should hit message size limit
        50_000_000, // 50MB - should definitely fail
    ];
    
    for size in payload_sizes {
        let large_data = create_large_test_data(size / 1024);
        let mut args = HashMap::new();
        args.insert("test_arg".to_string(), serde_json::Value::String(large_data));
        
        let result = client.send_command(
            "test-command",
            Some(args),
            Some(std::time::Duration::from_millis(100)),
        ).await;
        
        if size > 5_000_000 {
            // Should fail for large payloads
            assert!(result.is_err(), "Should reject large payload of {} bytes", size);
            
            match result.unwrap_err() {
                UnixSockApiError::ResourceLimit(_) | 
                UnixSockApiError::MessageTooLarge(_, _) |
                UnixSockApiError::ConnectionError(_) |
                UnixSockApiError::CommandTimeout(_, _) => {
                    // Expected errors
                },
                err => panic!("Unexpected error for large payload: {:?}", err),
            }
        }
    }
}

#[tokio::test]
async fn test_repeated_large_payload_attacks() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap();
    
    // Attempt 5 large payload attacks
    for i in 0..5 {
        let large_data = create_large_test_data(1000); // 1MB each
        let mut args = HashMap::new();
        args.insert("test_arg".to_string(), serde_json::Value::String(large_data));
        
        let result = client.send_command(
            "test-command",
            Some(args),
            Some(std::time::Duration::from_millis(100)),
        ).await;
        
        // Each should be handled gracefully without system impact
        match result {
            Ok(_) => {},
            Err(UnixSockApiError::ConnectionError(_)) => {},
            Err(UnixSockApiError::CommandTimeout(_, _)) => {},
            Err(UnixSockApiError::ResourceLimit(_)) => {},
            Err(err) => panic!("Iteration {}: Unexpected error: {:?}", i, err),
        }
    }
}

#[tokio::test]
async fn test_connection_pool_exhaustion() {
    let api_spec = create_test_api_spec();
    let mut config = create_test_config();
    config.max_concurrent_connections = 2; // Very low limit
    let socket_path = create_valid_socket_path();
    
    // This test verifies that connection pool limits are enforced
    let client = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap();
    
    // Try to exceed connection pool
    let args = create_test_args();
    
    let mut tasks = Vec::new();
    for _ in 0..5 {
        let client_clone = &client;
        let args_clone = args.clone();
        
        tasks.push(async move {
            client_clone.send_command(
                "test-command",
                Some(args_clone),
                Some(std::time::Duration::from_millis(100)),
            ).await
        });
    }
    
    // At least some should fail due to connection limits
    let results = futures::future::join_all(tasks).await;
    let error_count = results.iter().filter(|r| r.is_err()).count();
    
    // We expect some failures due to connection limits or timeouts
    assert!(error_count > 0, "Should have some failures due to resource limits");
}

#[tokio::test]
async fn test_rapid_connection_attempts() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Attempt 100 rapid connection attempts
    let mut tasks = Vec::new();
    
    for _ in 0..100 {
        let socket_path_clone = socket_path.clone();
        let api_spec_clone = api_spec.clone();
        let config_clone = config.clone();
        
        tasks.push(async move {
            UnixSockApiDatagramClient::new(
                socket_path_clone,
                "test-channel".to_string(),
                Some(api_spec_clone),
                config_clone,
            )
        });
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // All should either succeed or fail gracefully
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(_) => {}, // Success is fine
            Err(UnixSockApiError::ResourceLimit(_)) => {}, // Expected under load
            Err(err) => {
                // Should not crash or cause system issues
                assert!(!format!("{:?}", err).contains("system"), 
                       "Attempt {}: System error: {:?}", i, err);
            }
        }
    }
}

#[tokio::test]
async fn test_insecure_configuration_prevention() {
    let api_spec = create_test_api_spec();
    let socket_path = create_valid_socket_path();
    
    // Test with insecure configuration values
    let insecure_config = UnixSockApiClientConfig {
        max_concurrent_connections: 0,  // Invalid
        max_message_size: 0,            // Invalid
        connection_timeout: std::time::Duration::from_secs(0), // Invalid
        max_pending_commands: 0,        // Invalid
        max_command_handlers: 0,        // Invalid
        enable_resource_monitoring: false,
        max_channel_name_length: 0,     // Invalid
        max_command_name_length: 0,     // Invalid
        max_args_data_size: 0,          // Invalid
    };
    
    let result = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        insecure_config,
    );
    
    assert!(result.is_err(), "Should reject insecure configuration");
    
    match result.unwrap_err() {
        UnixSockApiError::ValidationError(_) => {}, // Expected
        err => panic!("Expected ValidationError, got: {:?}", err),
    }
}

#[tokio::test]
async fn test_extreme_configuration_values() {
    let api_spec = create_test_api_spec();
    let socket_path = create_valid_socket_path();
    
    // Test with extreme but valid configuration values
    let extreme_config = UnixSockApiClientConfig {
        max_concurrent_connections: 1_000_000,
        max_message_size: u32::MAX as usize,
        connection_timeout: std::time::Duration::from_secs(86400), // 24 hours
        max_pending_commands: 1_000_000,
        max_command_handlers: 1_000_000,
        enable_resource_monitoring: true,
        max_channel_name_length: 10_000,
        max_command_name_length: 10_000,
        max_args_data_size: 1_000_000_000, // 1GB
    };
    
    let result = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        extreme_config,
    );
    
    // Should handle extreme values gracefully
    match result {
        Ok(_) => {}, // Should work with extreme but valid values
        Err(err) => {
            // If it fails, should be for resource reasons, not crashes
            assert!(!format!("{:?}", err).contains("panic"));
        }
    }
}

#[tokio::test]
async fn test_validation_bypass_attempts() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap();
    
    // SOCK_DGRAM doesn't use register_command_handler - test command validation instead
    // Attempt to send commands beyond rate limits
    for i in 0..60 {
        let mut args = HashMap::new();
        args.insert("test_arg".to_string(), serde_json::Value::String(format!("test_{}", i)));
        
        let result = client.send_command(
            "test-command",
            Some(args),
            Some(std::time::Duration::from_millis(10)),
        ).await;
        
        // Should handle rapid commands gracefully
        match result {
            Ok(_) => {}, // Success is fine
            Err(UnixSockApiError::ResourceLimit(_)) => {}, // Expected under load
            Err(UnixSockApiError::CommandTimeout(_, _)) => {}, // Expected with no server
            Err(UnixSockApiError::ConnectionError(_)) => {}, // Expected with no server
            Err(err) => {
                // Should not crash or cause system issues
                assert!(!format!("{:?}", err).contains("panic"), 
                       "Command {}: Unexpected error: {:?}", i, err);
            }
        }
    }
}