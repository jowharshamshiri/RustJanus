use rust_janus::protocol::{MessageFraming, MessageFramingMessage, JanusCommand, JanusResponse};
use std::collections::HashMap;

#[tokio::test]
async fn test_message_framing_encode_message() {
    let framing = MessageFraming::new();
    
    // Test command encoding
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Command(command);
    let encoded = framing.encode_message(message).unwrap();
    
    assert!(encoded.len() > 4); // At least length prefix + content
    
    // Check length prefix (first 4 bytes)
    let message_length = u32::from_be_bytes([encoded[0], encoded[1], encoded[2], encoded[3]]) as usize;
    assert_eq!(message_length, encoded.len() - 4);
    
    // Test response encoding
    let response = JanusResponse::success(
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        "test-service".to_string(),
        Some(serde_json::json!({"pong": true})),
    );
    
    let message = MessageFramingMessage::Response(response);
    let encoded = framing.encode_message(message).unwrap();
    
    assert!(encoded.len() > 4);
}

#[tokio::test]
async fn test_message_framing_encode_large_message() {
    let framing = MessageFraming::new();
    
    // Create a command with very large args
    let mut large_args = HashMap::new();
    large_args.insert("data".to_string(), serde_json::Value::String("x".repeat(20 * 1024 * 1024))); // 20MB
    
    let command = JanusCommand::new(
        "test-service".to_string(),
        "large".to_string(),
        Some(large_args),
        None,
    );
    
    let message = MessageFramingMessage::Command(command);
    let result = framing.encode_message(message);
    
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(err.code, "MESSAGE_TOO_LARGE");
    }
}

#[tokio::test]
async fn test_message_framing_decode_message() {
    let framing = MessageFraming::new();
    
    // Test command decoding
    let original_command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Command(original_command.clone());
    let encoded = framing.encode_message(message).unwrap();
    
    let (decoded, remaining) = framing.decode_message(&encoded).unwrap();
    assert!(remaining.is_empty());
    
    if let MessageFramingMessage::Command(decoded_command) = decoded {
        assert_eq!(decoded_command.channelId, original_command.channelId);
        assert_eq!(decoded_command.command, original_command.command);
        assert_eq!(decoded_command.id, original_command.id);
    } else {
        panic!("Expected Command message");
    }
    
    // Test response decoding
    let original_response = JanusResponse::success(
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        "test-service".to_string(),
        Some(serde_json::json!({"pong": true})),
    );
    
    let message = MessageFramingMessage::Response(original_response.clone());
    let encoded = framing.encode_message(message).unwrap();
    
    let (decoded, remaining) = framing.decode_message(&encoded).unwrap();
    assert!(remaining.is_empty());
    
    if let MessageFramingMessage::Response(decoded_response) = decoded {
        assert_eq!(decoded_response.commandId, original_response.commandId);
        assert_eq!(decoded_response.success, original_response.success);
    } else {
        panic!("Expected Response message");
    }
}

#[tokio::test]
async fn test_message_framing_multiple_messages() {
    let framing = MessageFraming::new();
    
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let response = JanusResponse::success(
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        "test-service".to_string(),
        Some(serde_json::json!({"pong": true})),
    );
    
    let encoded1 = framing.encode_message(MessageFramingMessage::Command(command)).unwrap();
    let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&encoded1);
    combined.extend_from_slice(&encoded2);
    
    // Extract first message
    let (message1, remaining) = framing.decode_message(&combined).unwrap();
    assert!(matches!(message1, MessageFramingMessage::Command(_)));
    
    // Extract second message
    let (message2, final_remaining) = framing.decode_message(&remaining).unwrap();
    assert!(matches!(message2, MessageFramingMessage::Response(_)));
    assert!(final_remaining.is_empty());
}

#[tokio::test]
async fn test_message_framing_decode_errors() {
    let framing = MessageFraming::new();
    
    // Test incomplete length prefix
    let short_buffer = vec![0x00, 0x00]; // Only 2 bytes
    let result = framing.decode_message(&short_buffer);
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(err.code, "INCOMPLETE_LENGTH_PREFIX");
    }
    
    // Test incomplete message
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let encoded = framing.encode_message(MessageFramingMessage::Command(command)).unwrap();
    let truncated = &encoded[..encoded.len()-10]; // Remove last 10 bytes
    
    let result = framing.decode_message(truncated);
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(err.code, "INCOMPLETE_MESSAGE");
    }
    
    // Test zero-length message
    let zero_length_buffer = vec![0x00, 0x00, 0x00, 0x00]; // 0 length
    let result = framing.decode_message(&zero_length_buffer);
    assert!(result.is_err());
    if let Err(err) = result {
        assert_eq!(err.code, "ZERO_LENGTH_MESSAGE");
    }
}

#[tokio::test]
async fn test_message_framing_extract_messages() {
    let framing = MessageFraming::new();
    
    // Test multiple complete messages
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let response = JanusResponse::success(
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        "test-service".to_string(),
        Some(serde_json::json!({"pong": true})),
    );
    
    let encoded1 = framing.encode_message(MessageFramingMessage::Command(command)).unwrap();
    let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&encoded1);
    combined.extend_from_slice(&encoded2);
    
    let (messages, remaining) = framing.extract_messages(&combined).unwrap();
    
    assert_eq!(messages.len(), 2);
    assert!(remaining.is_empty());
    assert!(matches!(messages[0], MessageFramingMessage::Command(_)));
    assert!(matches!(messages[1], MessageFramingMessage::Response(_)));
}

#[tokio::test]
async fn test_message_framing_partial_messages() {
    let framing = MessageFraming::new();
    
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let response = JanusResponse::success(
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        "test-service".to_string(),
        Some(serde_json::json!({"pong": true})),
    );
    
    let encoded1 = framing.encode_message(MessageFramingMessage::Command(command)).unwrap();
    let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&encoded1);
    combined.extend_from_slice(&encoded2);
    
    // Take only part of the second message
    let partial = &combined[..encoded1.len() + 10];
    
    let (messages, remaining) = framing.extract_messages(partial).unwrap();
    
    assert_eq!(messages.len(), 1);
    assert_eq!(remaining.len(), 10); // Partial second message
    assert!(matches!(messages[0], MessageFramingMessage::Command(_)));
}

#[tokio::test]
async fn test_message_framing_empty_buffer() {
    let framing = MessageFraming::new();
    
    let (messages, remaining) = framing.extract_messages(&[]).unwrap();
    
    assert_eq!(messages.len(), 0);
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn test_message_framing_partial_length_prefix() {
    let framing = MessageFraming::new();
    
    let partial = vec![0x00, 0x00]; // Incomplete length prefix
    
    let (messages, remaining) = framing.extract_messages(&partial).unwrap();
    
    assert_eq!(messages.len(), 0);
    assert_eq!(remaining, partial);
}

#[tokio::test]
async fn test_message_framing_calculate_framed_size() {
    let framing = MessageFraming::new();
    
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Command(command.clone());
    let size = framing.calculate_framed_size(message.clone()).unwrap();
    let encoded = framing.encode_message(message).unwrap();
    
    assert_eq!(size, encoded.len());
}

#[tokio::test]
async fn test_message_framing_direct_message() {
    let framing = MessageFraming::new();
    
    // Test direct message encoding
    let command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Command(command.clone());
    let direct_encoded = framing.encode_direct_message(message.clone()).unwrap();
    
    assert!(direct_encoded.len() > 4);
    
    // Should be smaller than envelope version (no envelope overhead)
    let envelope_encoded = framing.encode_message(message).unwrap();
    assert!(direct_encoded.len() < envelope_encoded.len());
    
    // Test direct message decoding
    let (decoded, remaining) = framing.decode_direct_message(&direct_encoded).unwrap();
    assert!(remaining.is_empty());
    
    if let MessageFramingMessage::Command(decoded_command) = decoded {
        assert_eq!(decoded_command.channelId, command.channelId);
        assert_eq!(decoded_command.command, command.command);
        assert_eq!(decoded_command.id, command.id);
    } else {
        panic!("Expected Command message");
    }
}

#[tokio::test]
async fn test_message_framing_direct_roundtrip() {
    let framing = MessageFraming::new();
    
    // Test command roundtrip
    let original_command = JanusCommand::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Command(original_command.clone());
    let encoded = framing.encode_direct_message(message).unwrap();
    let (decoded, _) = framing.decode_direct_message(&encoded).unwrap();
    
    if let MessageFramingMessage::Command(decoded_command) = decoded {
        // Compare JSON representations for deep equality
        let original_json = serde_json::to_string(&original_command).unwrap();
        let decoded_json = serde_json::to_string(&decoded_command).unwrap();
        assert_eq!(original_json, decoded_json);
    } else {
        panic!("Expected Command message");
    }
    
    // Test response roundtrip
    let original_response = JanusResponse::success(
        "550e8400-e29b-41d4-a716-446655440000".to_string(),
        "test-service".to_string(),
        Some(serde_json::json!({"pong": true})),
    );
    
    let message = MessageFramingMessage::Response(original_response.clone());
    let encoded = framing.encode_direct_message(message).unwrap();
    let (decoded, _) = framing.decode_direct_message(&encoded).unwrap();
    
    if let MessageFramingMessage::Response(decoded_response) = decoded {
        // Compare JSON representations for deep equality
        let original_json = serde_json::to_string(&original_response).unwrap();
        let decoded_json = serde_json::to_string(&decoded_response).unwrap();
        assert_eq!(original_json, decoded_json);
    } else {
        panic!("Expected Response message");
    }
}