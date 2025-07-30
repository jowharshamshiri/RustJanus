use rust_janus::*;

mod test_utils;
use test_utils::*;

/// Protocol Tests - SOCK_DGRAM Architecture
/// Tests datagram message structure, JSON serialization, and data integrity

#[tokio::test]
async fn test_datagram_message_structure() {
    // Test SocketCommand serialization for SOCK_DGRAM
    let socket_command = SocketCommand {
        id: "test-id".to_string(),
        channelId: "test-channel".to_string(),
        command: "test-command".to_string(),
        reply_to: Some("/tmp/response.sock".to_string()),
        args: Some(create_test_args()),
        timeout: Some(30.0),
        timestamp: 1672531200.0, // 2023-01-01 00:00:00 UTC
    };
    
    // Test JSON serialization
    let json_str = serde_json::to_string(&socket_command).unwrap();
    assert!(json_str.contains("test-id"));
    assert!(json_str.contains("test-channel"));
    assert!(json_str.contains("reply_to"));
    
    // Test deserialization
    let deserialized: SocketCommand = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.id, "test-id");
    assert_eq!(deserialized.channelId, "test-channel");
    assert_eq!(deserialized.reply_to, Some("/tmp/response.sock".to_string()));
}

#[tokio::test]
async fn test_datagram_size_validation() {
    // Test datagram size limits for SOCK_DGRAM
    let small_args = create_test_args();
    let large_args = create_large_test_data_map(1000); // Large argument map
    
    let test_cases = vec![
        ("small", small_args),
        ("large", large_args),
    ];
    
    for (name, args) in test_cases {
        let socket_command = SocketCommand {
            id: "test-id".to_string(),
            channelId: "test-channel".to_string(),
            command: "test-command".to_string(),
            reply_to: Some("/tmp/response.sock".to_string()),
            args: Some(args),
            timeout: Some(30.0),
            timestamp: 1672531200.0, // 2023-01-01 00:00:00 UTC
        };
        
        let json_str = serde_json::to_string(&socket_command).unwrap();
        let datagram_size = json_str.len();
        
        // SOCK_DGRAM has practical size limits (typically 64KB for Unix domain sockets)
        assert!(datagram_size > 0, "Datagram {} should have content", name);
        
        if datagram_size > 65536 {
            println!("Warning: Datagram {} size {} exceeds typical SOCK_DGRAM limits", name, datagram_size);
        }
    }
}

#[tokio::test]
async fn test_datagram_json_validation() {
    // Test valid SOCK_DGRAM command
    let valid_command = SocketCommand {
        id: "test-id".to_string(),
        channelId: "test-channel".to_string(),
        command: "test-command".to_string(),
        reply_to: Some("/tmp/response.sock".to_string()),
        args: Some(create_test_args()),
        timeout: Some(30.0),
        timestamp: 1672531200.0, // 2023-01-01 00:00:00 UTC
    };
    
    // Test JSON serialization/deserialization
    let json_str = serde_json::to_string(&valid_command).unwrap();
    let deserialized: SocketCommand = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.id, valid_command.id);
    assert_eq!(deserialized.channelId, valid_command.channelId);
    
    // Test malformed JSON patterns
    let malformed_json_patterns = vec![
        r#"{"id": "test""#,  // Incomplete JSON
        r#"{"invalid": "structure"}"#,  // Missing required fields
        r#"{"id": 123}"#,  // Wrong field type
    ];
    
    for (i, malformed_json) in malformed_json_patterns.iter().enumerate() {
        let result: std::result::Result<SocketCommand, _> = serde_json::from_str(malformed_json);
        assert!(result.is_err(), "Malformed JSON {} should fail deserialization", i);
    }
}

#[tokio::test]
async fn test_socket_response_structure() {
    // Test SocketResponse for SOCK_DGRAM
    let socket_response = SocketResponse {
        commandId: "cmd-123".to_string(),
        channelId: "test-channel".to_string(),
        success: true,
        result: Some(serde_json::Value::Object({
            let mut result = serde_json::Map::new();
            result.insert("data".to_string(), serde_json::Value::String("response_data".to_string()));
            result
        })),
        error: None,
        timestamp: 1672531201.0, // 2023-01-01 00:00:01 UTC
    };
    
    // Test JSON serialization
    let json_str = serde_json::to_string(&socket_response).unwrap();
    assert!(json_str.contains("cmd-123"));
    assert!(json_str.contains("test-channel"));
    assert!(json_str.contains("response_data"));
    
    // Test deserialization
    let deserialized: SocketResponse = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.commandId, "cmd-123");
    assert_eq!(deserialized.channelId, "test-channel");
    assert_eq!(deserialized.success, true);
}

#[tokio::test] 
async fn test_timestamp_values() {
    // Test different Unix timestamp values for SOCK_DGRAM compatibility
    let timestamps = vec![
        1672531200.0, // 2023-01-01 00:00:00 UTC
        1672576245.123, // 2023-01-01 12:30:45.123 UTC  
        1703980799.0, // 2023-12-31 23:59:59 UTC
    ];
    
    for timestamp in timestamps {
        let command = SocketCommand {
            id: "test-id".to_string(),
            channelId: "test-channel".to_string(),
            command: "test-command".to_string(),
            reply_to: None,
            args: None,
            timeout: Some(30.0),
            timestamp,
        };
        
        // Should serialize and deserialize successfully
        let json_str = serde_json::to_string(&command).unwrap();
        let deserialized: SocketCommand = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.timestamp, timestamp);
    }
}