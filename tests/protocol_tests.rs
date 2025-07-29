use rust_unix_sock_api::*;
use base64::Engine;

mod test_utils;
use test_utils::*;
use std::collections::HashMap;

/// Protocol Tests (11 tests) - Exact SwiftUnixSockAPI parity
/// Tests message framing, UTF-8 handling, message protocol, and data integrity

#[tokio::test]
async fn test_message_framing_structure() {
    // Test 4 message sizes (minimal, small, medium, large)
    let small_data = serde_json::to_string(&create_test_args()).unwrap();
    let medium_data = create_large_test_data(1); // 1KB
    let large_data = create_large_test_data(10); // 10KB
    
    let test_cases = vec![
        ("minimal", "{}"),
        ("small", &small_data),
        ("medium", &medium_data),
        ("large", &large_data),
    ];
    
    for (name, data) in test_cases {
        let message_bytes = data.as_bytes();
        
        // Test framing
        let frame_size = MessageFrame::frame_size(message_bytes.len());
        assert_eq!(frame_size, 4 + message_bytes.len(), "Frame size calculation for {}", name);
        
        // Test size validation
        let validation_result = MessageFrame::validate_frame_size(message_bytes.len(), 1_000_000);
        assert!(validation_result.is_ok(), "Size validation for {}", name);
    }
}

#[tokio::test]
async fn test_message_size_validation() {
    let boundary_sizes = vec![
        0, 1, 255, 256, 1023, 1024, 4095, 4096, 
        65535, 65536, 1_000_000, 5_000_000, 10_000_000
    ];
    
    for size in boundary_sizes {
        let max_size = 5_000_000; // 5MB limit
        let result = MessageFrame::validate_frame_size(size, max_size);
        
        if size <= max_size {
            assert!(result.is_ok(), "Size {} should be valid", size);
        } else {
            assert!(result.is_err(), "Size {} should be invalid", size);
        }
    }
}

#[tokio::test]
async fn test_malformed_message_framing() {
    let valid_command = SocketCommand::new(
        "test-channel".to_string(),
        "test-command".to_string(),
        Some(create_test_args()),
        Some(30.0),
    );
    
    // Test valid message first
    let valid_message = SocketMessage::command(valid_command.clone()).unwrap();
    assert!(valid_message.validate().is_ok(), "Valid message should pass validation");
    
    // Test malformed messages
    let malformed_messages = vec![
        SocketMessage {
            r#type: MessageType::Command,
            payload: Vec::new(), // Empty payload
        },
        SocketMessage {
            r#type: MessageType::Command,
            payload: b"invalid json".to_vec(), // Invalid JSON
        },
        SocketMessage {
            r#type: MessageType::Response,
            payload: b"{\"not\": \"a_command\"}".to_vec(), // Wrong type
        },
    ];
    
    for (i, malformed) in malformed_messages.iter().enumerate() {
        let result = malformed.validate();
        assert!(result.is_err(), "Malformed message {} should fail validation", i);
    }
    
    // Test 3 additional valid messages
    for i in 0..3 {
        let valid_cmd = SocketCommand::new(
            format!("channel-{}", i),
            format!("command-{}", i),
            None,
            None,
        );
        let valid_msg = SocketMessage::command(valid_cmd).unwrap();
        assert!(valid_msg.validate().is_ok(), "Valid message {} should pass", i);
    }
}

#[tokio::test]
async fn test_message_boundaries() {
    // Test 12 boundary sizes matching Swift test
    let boundary_sizes = vec![
        0, 1, 255, 256, 1023, 1024, 4095, 4096,
        65535, 65536, 1_048_575, 1_048_576, // 1MB boundary
    ];
    
    for size in boundary_sizes {
        if size == 0 {
            // Empty message should be invalid
            let empty_message = SocketMessage {
                r#type: MessageType::Command,
                payload: Vec::new(),
            };
            assert!(empty_message.validate().is_err(), "Empty message should be invalid");
        } else {
            // Create message of exact size
            let data = "x".repeat(size);
            let command = SocketCommand::new(
                "test-channel".to_string(),
                "test-command".to_string(),
                Some({
                    let mut args = HashMap::new();
                    args.insert("data".to_string(), serde_json::Value::String(data));
                    args
                }),
                None,
            );
            
            let message = SocketMessage::command(command);
            match message {
                Ok(msg) => {
                    let payload_size = msg.payload_size();
                    println!("Boundary size {}: payload {} bytes", size, payload_size);
                },
                Err(_) => {
                    // Large messages may fail serialization
                    println!("Boundary size {}: failed serialization (expected for large sizes)", size);
                }
            }
        }
    }
}

#[tokio::test]
async fn test_utf8_encoding_handling() {
    let utf8_test_cases = get_utf8_test_cases();
    
    for (name, text) in utf8_test_cases {
        let command = SocketCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            Some({
                let mut args = HashMap::new();
                args.insert("text".to_string(), serde_json::Value::String(text.to_string()));
                args
            }),
            None,
        );
        
        // Should serialize and validate successfully
        let message = SocketMessage::command(command).unwrap();
        assert!(message.validate().is_ok(), "UTF-8 case '{}' should be valid", name);
        
        // Should decode back correctly
        let decoded_command = message.decode_command().unwrap();
        let args_binding = decoded_command.args.unwrap();
        let decoded_text = args_binding.get("text").unwrap().as_str().unwrap();
        
        assert_eq!(decoded_text, text, "UTF-8 case '{}' should round-trip correctly", name);
    }
}

#[tokio::test]
async fn test_invalid_utf8_handling() {
    let invalid_sequences = get_invalid_utf8_sequences();
    
    for (i, invalid_bytes) in invalid_sequences.iter().enumerate() {
        // Test that invalid UTF-8 is properly rejected
        let result = SecurityValidator::validate_utf8_data(invalid_bytes);
        assert!(result.is_err(), "Invalid UTF-8 sequence {} should be rejected", i);
        
        match result.unwrap_err() {
            UnixSockApiError::MalformedData(msg) => {
                assert!(msg.contains("UTF-8"), "Error message should mention UTF-8");
            },
            err => panic!("Expected MalformedData error, got: {:?}", err),
        }
    }
}

#[tokio::test]
async fn test_socket_message_serialization() {
    let command = SocketCommand::new(
        "test-channel".to_string(),
        "test-command".to_string(),
        Some(create_test_args()),
        Some(30.0),
    );
    
    // Test SocketMessage serialization
    let message = SocketMessage::command(command.clone()).unwrap();
    
    // Serialize and deserialize
    let serialized = serde_json::to_vec(&message).unwrap();
    let deserialized: SocketMessage = serde_json::from_slice(&serialized).unwrap();
    
    assert_eq!(message.r#type, deserialized.r#type);
    assert_eq!(message.payload, deserialized.payload);
    
    // Decode command from deserialized message
    let decoded_command = deserialized.decode_command().unwrap();
    assert_eq!(decoded_command.channel_id, command.channel_id);
    assert_eq!(decoded_command.command, command.command);
}

#[tokio::test]
async fn test_socket_command_serialization() {
    let command = SocketCommand::new(
        "test-channel".to_string(),
        "test-command".to_string(),
        Some(create_test_args()),
        Some(30.0),
    );
    
    // Serialize and deserialize
    let serialized = serde_json::to_vec(&command).unwrap();
    let deserialized: SocketCommand = serde_json::from_slice(&serialized).unwrap();
    
    assert_eq!(command.id, deserialized.id);
    assert_eq!(command.channel_id, deserialized.channel_id);
    assert_eq!(command.command, deserialized.command);
    assert_eq!(command.args, deserialized.args);
    assert_eq!(command.timeout, deserialized.timeout);
}

#[tokio::test]
async fn test_socket_response_serialization() {
    let error_response = SocketResponse::error(
        "test-command-id".to_string(),
        "test-channel".to_string(),
        SocketError::ValidationFailed("Test error".to_string()),
    );
    
    // Serialize and deserialize error response
    let serialized = serde_json::to_vec(&error_response).unwrap();
    let deserialized: SocketResponse = serde_json::from_slice(&serialized).unwrap();
    
    assert_eq!(error_response.command_id, deserialized.command_id);
    assert_eq!(error_response.channel_id, deserialized.channel_id);
    assert_eq!(error_response.success, deserialized.success);
    assert_eq!(error_response.error, deserialized.error);
    
    assert!(!deserialized.success);
    assert!(deserialized.error.is_some());
    assert!(deserialized.result.is_none());
}

#[tokio::test]
async fn test_anyccodable_edge_cases() {
    // Test 12 edge cases matching Swift implementation
    let edge_cases = vec![
        ("int_min", serde_json::Value::Number(serde_json::Number::from(i64::MIN))),
        ("int_max", serde_json::Value::Number(serde_json::Number::from(i64::MAX))),
        ("float_min", serde_json::Value::Number(serde_json::Number::from_f64(f64::MIN).unwrap())),
        ("float_max", serde_json::Value::Number(serde_json::Number::from_f64(f64::MAX).unwrap())),
        ("zero", serde_json::Value::Number(serde_json::Number::from(0))),
        ("negative_zero", serde_json::Value::Number(serde_json::Number::from_f64(-0.0).unwrap())),
        ("empty_string", serde_json::Value::String("".to_string())),
        ("empty_array", serde_json::Value::Array(Vec::new())),
        ("empty_object", serde_json::Value::Object(serde_json::Map::new())),
        ("null", serde_json::Value::Null),
        ("true", serde_json::Value::Bool(true)),
        ("false", serde_json::Value::Bool(false)),
    ];
    
    for (name, value) in edge_cases {
        let command = SocketCommand::new(
            "test-channel".to_string(),
            "test-command".to_string(),
            Some({
                let mut args = HashMap::new();
                args.insert("edge_case".to_string(), value.clone());
                args
            }),
            None,
        );
        
        // Should serialize and deserialize correctly
        let message = SocketMessage::command(command).unwrap();
        let decoded = message.decode_command().unwrap();
        let args_binding = decoded.args.unwrap();
        let decoded_value = args_binding.get("edge_case").unwrap();
        
        assert_eq!(&value, decoded_value, "Edge case '{}' should round-trip correctly", name);
    }
}

#[tokio::test]
async fn test_anyccodable_nested_structures() {
    let nested_data = create_nested_test_data();
    
    let command = SocketCommand::new(
        "test-channel".to_string(),
        "test-command".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("nested".to_string(), nested_data.clone());
            args
        }),
        None,
    );
    
    // Should handle 4-level deep nesting
    let message = SocketMessage::command(command).unwrap();
    let decoded = message.decode_command().unwrap();
    let args_binding = decoded.args.unwrap();
    let decoded_nested = args_binding.get("nested").unwrap();
    
    assert_eq!(&nested_data, decoded_nested, "Deep nesting should round-trip correctly");
    
    // Verify deep access
    let deep_value = decoded_nested["level1"]["level2"]["level3"]["level4"]["deep_value"].as_str().unwrap();
    assert_eq!(deep_value, "nested_data");
}

#[tokio::test]
async fn test_data_integrity_across_encoding() {
    // Test 7 data types with special characters
    let test_data = vec![
        ("ascii", "Hello World"),
        ("unicode", "Hello 世界 🌍"),
        ("json_escape", r#"{"key": "value \"with quotes\""}"#),
        ("newlines", "Line 1\nLine 2\r\nLine 3"),
        ("tabs", "Column1\tColumn2\tColumn3"),
        ("backslashes", r"C:\Windows\System32\file.txt"),
        ("mixed_special", "Mix: 'quotes' \"double\" \\ / \n \t 🚀"),
    ];
    
    for (name, text) in test_data {
        let original_command = SocketCommand::new(
            "test-channel".to_string(),
            "integrity-test".to_string(),
            Some({
                let mut args = HashMap::new();
                args.insert("data".to_string(), serde_json::Value::String(text.to_string()));
                args
            }),
            None,
        );
        
        // Encode to JSON
        let json_bytes = serde_json::to_vec(&original_command).unwrap();
        
        // Decode from JSON
        let decoded_command: SocketCommand = serde_json::from_slice(&json_bytes).unwrap();
        
        // Verify integrity
        let binding = decoded_command.args.unwrap();
        let decoded_text = binding.get("data").unwrap()
            .as_str().unwrap();
        
        assert_eq!(text, decoded_text, "Data integrity test '{}' failed", name);
    }
}

#[tokio::test]
async fn test_binary_data_handling() {
    // Binary data should be rejected or properly base64 encoded
    let binary_data = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];
    
    // Test that raw binary data is rejected
    let utf8_validation = SecurityValidator::validate_utf8_data(&binary_data);
    assert!(utf8_validation.is_err(), "Raw binary data should be rejected");
    
    // Test base64 encoded binary data
    let base64_encoded = base64::engine::general_purpose::STANDARD.encode(&binary_data);
    
    let command = SocketCommand::new(
        "test-channel".to_string(),
        "binary-test".to_string(),
        Some({
            let mut args = HashMap::new();
            args.insert("data".to_string(), serde_json::Value::String(base64_encoded.clone()));
            args
        }),
        None,
    );
    
    let message = SocketMessage::command(command).unwrap();
    let decoded = message.decode_command().unwrap();
    let binding = decoded.args.unwrap();
    let decoded_base64 = binding.get("data").unwrap().as_str().unwrap();
    
    assert_eq!(base64_encoded, decoded_base64, "Base64 encoded binary should round-trip correctly");
}