use rust_janus::protocol::{MessageFraming, MessageFramingMessage, JanusRequest, JanusResponse};
use std::collections::HashMap;

#[tokio::test]
async fn test_message_framing_encode_message() {
    let framing = MessageFraming::new();
    
    // Test request encoding
    let request = JanusRequest::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Request(request);
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
    
    // Create a request with very large args
    let mut large_args = HashMap::new();
    large_args.insert("data".to_string(), serde_json::Value::String("x".repeat(20 * 1024 * 1024))); // 20MB
    
    let request = JanusRequest::new(
        "test-service".to_string(),
        "large".to_string(),
        Some(large_args),
        None,
    );
    
    let message = MessageFramingMessage::Request(request);
    let result = framing.encode_message(message);
    
    assert!(result.is_err());
    if let Err(err) = result {
        // Validate error code instead of error string - should be JSONRPCError with MessageFramingError code
        assert_eq!(err.code, -32011); // MessageFramingError code 
    }
}

#[tokio::test]
async fn test_message_framing_decode_message() {
    let framing = MessageFraming::new();
    
    // Test request decoding
    let original_request = JanusRequest::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Request(original_request.clone());
    let encoded = framing.encode_message(message).unwrap();
    
    let (decoded, remaining) = framing.decode_message(&encoded).unwrap();
    assert!(remaining.is_empty());
    
    if let MessageFramingMessage::Request(decoded_request) = decoded {
        assert_eq!(decoded_request.channelId, original_request.channelId);
        assert_eq!(decoded_request.request, original_request.request);
        assert_eq!(decoded_request.id, original_request.id);
    } else {
        panic!("Expected Request message");
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
        assert_eq!(decoded_response.requestId, original_response.requestId);
        assert_eq!(decoded_response.success, original_response.success);
    } else {
        panic!("Expected Response message");
    }
}

#[tokio::test]
async fn test_message_framing_multiple_messages() {
    let framing = MessageFraming::new();
    
    let request = JanusRequest::new(
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
    
    let encoded1 = framing.encode_message(MessageFramingMessage::Request(request)).unwrap();
    let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&encoded1);
    combined.extend_from_slice(&encoded2);
    
    // Extract first message
    let (message1, remaining) = framing.decode_message(&combined).unwrap();
    assert!(matches!(message1, MessageFramingMessage::Request(_)));
    
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
        // Validate error code instead of error string - should be JSONRPCError with MessageFramingError code
        assert_eq!(err.code, -32011); // MessageFramingError code 
    }
    
    // Test incomplete message
    let request = JanusRequest::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let encoded = framing.encode_message(MessageFramingMessage::Request(request)).unwrap();
    let truncated = &encoded[..encoded.len()-10]; // Remove last 10 bytes
    
    let result = framing.decode_message(truncated);
    assert!(result.is_err());
    if let Err(err) = result {
        // Validate error code instead of error string - should be JSONRPCError with MessageFramingError code
        assert_eq!(err.code, -32011); // MessageFramingError code 
    }
    
    // Test zero-length message
    let zero_length_buffer = vec![0x00, 0x00, 0x00, 0x00]; // 0 length
    let result = framing.decode_message(&zero_length_buffer);
    assert!(result.is_err());
    if let Err(err) = result {
        // Validate error code instead of error string - should be JSONRPCError with MessageFramingError code
        assert_eq!(err.code, -32011); // MessageFramingError code 
    }
}

#[tokio::test]
async fn test_message_framing_extract_messages() {
    let framing = MessageFraming::new();
    
    // Test multiple complete messages
    let request = JanusRequest::new(
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
    
    let encoded1 = framing.encode_message(MessageFramingMessage::Request(request)).unwrap();
    let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&encoded1);
    combined.extend_from_slice(&encoded2);
    
    let (messages, remaining) = framing.extract_messages(&combined).unwrap();
    
    assert_eq!(messages.len(), 2);
    assert!(remaining.is_empty());
    assert!(matches!(messages[0], MessageFramingMessage::Request(_)));
    assert!(matches!(messages[1], MessageFramingMessage::Response(_)));
}

#[tokio::test]
async fn test_message_framing_partial_messages() {
    let framing = MessageFraming::new();
    
    let request = JanusRequest::new(
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
    
    let encoded1 = framing.encode_message(MessageFramingMessage::Request(request)).unwrap();
    let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
    
    let mut combined = Vec::new();
    combined.extend_from_slice(&encoded1);
    combined.extend_from_slice(&encoded2);
    
    // Take only part of the second message
    let partial = &combined[..encoded1.len() + 10];
    
    let (messages, remaining) = framing.extract_messages(partial).unwrap();
    
    assert_eq!(messages.len(), 1);
    assert_eq!(remaining.len(), 10); // Partial second message
    assert!(matches!(messages[0], MessageFramingMessage::Request(_)));
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
    
    let request = JanusRequest::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Request(request.clone());
    let size = framing.calculate_framed_size(message.clone()).unwrap();
    let encoded = framing.encode_message(message).unwrap();
    
    assert_eq!(size, encoded.len());
}

#[tokio::test]
async fn test_message_framing_direct_message() {
    let framing = MessageFraming::new();
    
    // Test direct message encoding
    let request = JanusRequest::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Request(request.clone());
    let direct_encoded = framing.encode_direct_message(message.clone()).unwrap();
    
    assert!(direct_encoded.len() > 4);
    
    // Should be smaller than envelope version (no envelope overhead)
    let envelope_encoded = framing.encode_message(message).unwrap();
    assert!(direct_encoded.len() < envelope_encoded.len());
    
    // Test direct message decoding
    let (decoded, remaining) = framing.decode_direct_message(&direct_encoded).unwrap();
    assert!(remaining.is_empty());
    
    if let MessageFramingMessage::Request(decoded_request) = decoded {
        assert_eq!(decoded_request.channelId, request.channelId);
        assert_eq!(decoded_request.request, request.request);
        assert_eq!(decoded_request.id, request.id);
    } else {
        panic!("Expected Request message");
    }
}

#[tokio::test]
async fn test_message_framing_direct_roundtrip() {
    let framing = MessageFraming::new();
    
    // Test request roundtrip
    let original_request = JanusRequest::new(
        "test-service".to_string(),
        "ping".to_string(),
        None,
        None,
    );
    
    let message = MessageFramingMessage::Request(original_request.clone());
    let encoded = framing.encode_direct_message(message).unwrap();
    let (decoded, _) = framing.decode_direct_message(&encoded).unwrap();
    
    if let MessageFramingMessage::Request(decoded_request) = decoded {
        // Compare JSON representations for deep equality
        let original_json = serde_json::to_string(&original_request).unwrap();
        let decoded_json = serde_json::to_string(&decoded_request).unwrap();
        assert_eq!(original_json, decoded_json);
    } else {
        panic!("Expected Request message");
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