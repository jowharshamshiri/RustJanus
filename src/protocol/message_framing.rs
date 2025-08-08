use crate::error::{JSONRPCError, JSONRPCErrorCode};
use crate::protocol::message_types::{JanusRequest, JanusResponse};
use serde::{Deserialize, Serialize};

const LENGTH_PREFIX_SIZE: usize = 4;
const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB default

// Legacy MessageFramingError eliminated - using JSONRPCError with MessageFramingError code

/// Socket message envelope for framing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SocketMessageEnvelope {
    #[serde(rename = "type")]
    pub message_type: String, // "request" or "response"
    pub payload: String,      // JSON payload as string
}

/// Message framing functionality with 4-byte length prefix
pub struct MessageFraming {}

impl MessageFraming {
    /// Create a new message framing instance
    pub fn new() -> Self {
        Self {}
    }

    /// Encode a message with 4-byte big-endian length prefix
    pub fn encode_message(&self, message: MessageFramingMessage) -> Result<Vec<u8>, JSONRPCError> {
        // Determine message type and serialize payload
        let (message_type, payload_bytes) = match message {
            MessageFramingMessage::Request(cmd) => {
                let payload = serde_json::to_vec(&cmd).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to marshal request payload: {}", e))))?;
                ("request".to_string(), payload)
            }
            MessageFramingMessage::Response(resp) => {
                let payload = serde_json::to_vec(&resp).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to marshal response payload: {}", e))))?;
                ("response".to_string(), payload)
            }
        };

        // Create envelope with JSON payload
        let envelope = SocketMessageEnvelope {
            message_type,
            payload: String::from_utf8(payload_bytes).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to convert payload to string: {}", e))))?,
        };

        // Serialize envelope to JSON
        let envelope_bytes = serde_json::to_vec(&envelope).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to marshal envelope: {}", e))))?;

        // Validate message size
        if envelope_bytes.len() > MAX_MESSAGE_SIZE {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Message size {} exceeds maximum {}", envelope_bytes.len(), MAX_MESSAGE_SIZE))));
        }

        // Create length prefix (4-byte big-endian)
        let mut result = Vec::with_capacity(LENGTH_PREFIX_SIZE + envelope_bytes.len());
        result.extend_from_slice(&(envelope_bytes.len() as u32).to_be_bytes());
        result.extend_from_slice(&envelope_bytes);

        Ok(result)
    }

    /// Decode a message from buffer with length prefix
    pub fn decode_message(&self, buffer: &[u8]) -> Result<(MessageFramingMessage, Vec<u8>), JSONRPCError> {
        // Check if we have at least the length prefix
        if buffer.len() < LENGTH_PREFIX_SIZE {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Buffer too small for length prefix: {} < {}", buffer.len(), LENGTH_PREFIX_SIZE))));
        }

        // Read message length from big-endian prefix
        let message_length = u32::from_be_bytes([
            buffer[0], buffer[1], buffer[2], buffer[3]
        ]) as usize;

        // Validate message length
        if message_length > MAX_MESSAGE_SIZE {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Message length {} exceeds maximum {}", message_length, MAX_MESSAGE_SIZE))));
        }

        if message_length == 0 {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some("Message length cannot be zero".to_string())));
        }

        // Check if we have the complete message
        let total_required = LENGTH_PREFIX_SIZE + message_length;
        if buffer.len() < total_required {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Buffer too small for complete message: {} < {}", buffer.len(), total_required))));
        }

        // Extract message data
        let message_buffer = &buffer[LENGTH_PREFIX_SIZE..LENGTH_PREFIX_SIZE + message_length];
        let remaining_buffer = buffer[LENGTH_PREFIX_SIZE + message_length..].to_vec();

        // Parse JSON envelope
        let envelope: SocketMessageEnvelope = serde_json::from_slice(message_buffer).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to parse message envelope JSON: {}", e))))?;

        // Validate envelope structure
        if envelope.message_type.is_empty() || envelope.payload.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some("Message envelope missing required fields (type, payload)".to_string())));
        }

        if envelope.message_type != "request" && envelope.message_type != "response" {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Invalid message type: {}", envelope.message_type))));
        }

        // Parse payload JSON directly
        let message = if envelope.message_type == "request" {
            let cmd: JanusRequest = serde_json::from_str(&envelope.payload).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to parse request payload JSON: {}", e))))?;
            
            // Validate request structure
            self.validate_request_structure(&cmd)?;
            MessageFramingMessage::Request(cmd)
        } else {
            let resp: JanusResponse = serde_json::from_str(&envelope.payload).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to parse response payload JSON: {}", e))))?;
            
            // Validate response structure
            self.validate_response_structure(&resp)?;
            MessageFramingMessage::Response(resp)
        };

        Ok((message, remaining_buffer))
    }

    /// Extract complete messages from a buffer, handling partial messages
    pub fn extract_messages(&self, buffer: &[u8]) -> Result<(Vec<MessageFramingMessage>, Vec<u8>), JSONRPCError> {
        let mut messages = Vec::new();
        let mut current_buffer = buffer.to_vec();

        while !current_buffer.is_empty() {
            match self.decode_message(&current_buffer) {
                Ok((message, remaining_buffer)) => {
                    messages.push(message);
                    current_buffer = remaining_buffer;
                }
                Err(e) => {
                    if e.code == JSONRPCErrorCode::MessageFramingError.code() {
                        // Check if it's a partial message error by looking at the details
                        if let Some(data) = &e.data {
                            if let Some(details) = &data.details {
                                if details.contains("Buffer too small") {
                                    // Not enough data for complete message, save remaining buffer
                                    break;
                                }
                            }
                        }
                    }
                    return Err(e);
                }
            }
        }

        Ok((messages, current_buffer))
    }

    /// Calculate the total size needed for a message when framed
    pub fn calculate_framed_size(&self, message: MessageFramingMessage) -> Result<usize, JSONRPCError> {
        let encoded = self.encode_message(message)?;
        Ok(encoded.len())
    }

    /// Create a direct JSON message for simple cases (without envelope)
    pub fn encode_direct_message(&self, message: MessageFramingMessage) -> Result<Vec<u8>, JSONRPCError> {
        // Serialize message to JSON
        let message_bytes = match message {
            MessageFramingMessage::Request(cmd) => {
                serde_json::to_vec(&cmd).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to marshal request: {}", e))))?
            }
            MessageFramingMessage::Response(resp) => {
                serde_json::to_vec(&resp).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to marshal response: {}", e))))?
            }
        };

        // Validate message size
        if message_bytes.len() > MAX_MESSAGE_SIZE {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Message size {} exceeds maximum {}", message_bytes.len(), MAX_MESSAGE_SIZE))));
        }

        // Create length prefix and combine
        let mut result = Vec::with_capacity(LENGTH_PREFIX_SIZE + message_bytes.len());
        result.extend_from_slice(&(message_bytes.len() as u32).to_be_bytes());
        result.extend_from_slice(&message_bytes);

        Ok(result)
    }

    /// Decode a direct JSON message (without envelope)
    pub fn decode_direct_message(&self, buffer: &[u8]) -> Result<(MessageFramingMessage, Vec<u8>), JSONRPCError> {
        // Check length prefix
        if buffer.len() < LENGTH_PREFIX_SIZE {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Buffer too small for length prefix: {} < {}", buffer.len(), LENGTH_PREFIX_SIZE))));
        }

        let message_length = u32::from_be_bytes([
            buffer[0], buffer[1], buffer[2], buffer[3]
        ]) as usize;
        let total_required = LENGTH_PREFIX_SIZE + message_length;

        if buffer.len() < total_required {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Buffer too small for complete message: {} < {}", buffer.len(), total_required))));
        }

        // Extract and parse message
        let message_buffer = &buffer[LENGTH_PREFIX_SIZE..LENGTH_PREFIX_SIZE + message_length];
        let remaining_buffer = buffer[LENGTH_PREFIX_SIZE + message_length..].to_vec();

        // Try to determine message type by looking for key fields
        let raw_value: serde_json::Value = serde_json::from_slice(message_buffer).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to parse message JSON: {}", e))))?;

        // Determine message type and parse accordingly
        let message = if raw_value.get("request").is_some() {
            let cmd: JanusRequest = serde_json::from_slice(message_buffer).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to parse request: {}", e))))?;
            MessageFramingMessage::Request(cmd)
        } else if raw_value.get("requestId").is_some() {
            let resp: JanusResponse = serde_json::from_slice(message_buffer).map_err(|e| JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some(format!("Failed to parse response: {}", e))))?;
            MessageFramingMessage::Response(resp)
        } else {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some("Cannot determine message type".to_string())));
        };

        Ok((message, remaining_buffer))
    }

    /// Validate request structure
    fn validate_request_structure(&self, cmd: &JanusRequest) -> Result<(), JSONRPCError> {
        if cmd.id.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some("Request missing required string field: id".to_string())));
        }
        if cmd.request.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some("Request missing required string field: request".to_string())));
        }
        Ok(())
    }

    /// Validate response structure
    fn validate_response_structure(&self, resp: &JanusResponse) -> Result<(), JSONRPCError> {
        if resp.request_id.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MessageFramingError, Some("Response missing required field: request_id".to_string())));
        }
        // PRIME DIRECTIVE: channelId is not part of JanusResponse format
        Ok(())
    }
}

/// Message enum for framing operations
#[derive(Debug, Clone)]
pub enum MessageFramingMessage {
    Request(JanusRequest),
    Response(JanusResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_request() {
        let framing = MessageFraming::new();
        
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        let message = MessageFramingMessage::Request(request.clone());
        let encoded = framing.encode_message(message).unwrap();
        
        assert!(encoded.len() > LENGTH_PREFIX_SIZE);
        
        let (decoded, remaining) = framing.decode_message(&encoded).unwrap();
        assert!(remaining.is_empty());
        
        if let MessageFramingMessage::Request(decoded_cmd) = decoded {
            assert_eq!(decoded_cmd.channelId, request.channelId);
            assert_eq!(decoded_cmd.request, request.request);
        } else {
            panic!("Expected Request message");
        }
    }

    #[test]
    fn test_encode_decode_response() {
        let framing = MessageFraming::new();
        
        let response = JanusResponse::success(
            "test-id".to_string(),
            "test-channel".to_string(),
            Some(serde_json::json!({"status": "ok"})),
        );
        
        let message = MessageFramingMessage::Response(response.clone());
        let encoded = framing.encode_message(message).unwrap();
        
        assert!(encoded.len() > LENGTH_PREFIX_SIZE);
        
        let (decoded, remaining) = framing.decode_message(&encoded).unwrap();
        assert!(remaining.is_empty());
        
        if let MessageFramingMessage::Response(decoded_resp) = decoded {
            assert_eq!(decoded_resp.requestId, response.requestId);
            assert_eq!(decoded_resp.success, response.success);
        } else {
            panic!("Expected Response message");
        }
    }

    #[test]
    fn test_extract_multiple_messages() {
        let framing = MessageFraming::new();
        
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        let response = JanusResponse::success(
            "test-id".to_string(),
            "test-channel".to_string(),
            None,
        );
        
        let encoded1 = framing.encode_message(MessageFramingMessage::Request(request)).unwrap();
        let encoded2 = framing.encode_message(MessageFramingMessage::Response(response)).unwrap();
        
        let mut combined = Vec::new();
        combined.extend_from_slice(&encoded1);
        combined.extend_from_slice(&encoded2);
        
        let (messages, remaining) = framing.extract_messages(&combined).unwrap();
        
        assert_eq!(messages.len(), 2);
        assert!(remaining.is_empty());
        
        match (&messages[0], &messages[1]) {
            (MessageFramingMessage::Request(_), MessageFramingMessage::Response(_)) => {},
            _ => panic!("Expected Request and Response messages"),
        }
    }

    #[test]
    fn test_partial_message_handling() {
        let framing = MessageFraming::new();
        
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        let encoded = framing.encode_message(MessageFramingMessage::Request(request)).unwrap();
        let partial = &encoded[..encoded.len()-10]; // Remove last 10 bytes
        
        let (messages, remaining) = framing.extract_messages(partial).unwrap();
        
        assert_eq!(messages.len(), 0);
        assert_eq!(remaining.len(), partial.len());
    }

    #[test]
    fn test_direct_message_encoding() {
        let framing = MessageFraming::new();
        
        let request = JanusRequest::new(
            "test-channel".to_string(),
            "test-request".to_string(),
            None,
            None,
        );
        
        let message = MessageFramingMessage::Request(request.clone());
        let direct_encoded = framing.encode_direct_message(message.clone()).unwrap();
        let envelope_encoded = framing.encode_message(message).unwrap();
        
        assert!(direct_encoded.len() < envelope_encoded.len());
        
        let (decoded, remaining) = framing.decode_direct_message(&direct_encoded).unwrap();
        assert!(remaining.is_empty());
        
        if let MessageFramingMessage::Request(decoded_cmd) = decoded {
            assert_eq!(decoded_cmd.channelId, request.channelId);
            assert_eq!(decoded_cmd.request, request.request);
        } else {
            panic!("Expected Request message");
        }
    }

    #[test]
    fn test_error_cases() {
        let framing = MessageFraming::new();
        
        // Test incomplete length prefix
        let short_buffer = [0x00, 0x00];
        let result = framing.decode_message(&short_buffer);
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.code, JSONRPCErrorCode::MessageFramingError.code());
            if let Some(data) = &e.data {
                if let Some(details) = &data.details {
                    assert!(details.contains("Buffer too small"));
                }
            }
        }
        
        // Test zero length message
        let zero_length = [0x00, 0x00, 0x00, 0x00];
        let result = framing.decode_message(&zero_length);
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e.code, JSONRPCErrorCode::MessageFramingError.code());
            if let Some(data) = &e.data {
                if let Some(details) = &data.details {
                    assert!(details.contains("Message length cannot be zero"));
                }
            }
        }
    }
}