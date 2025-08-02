use rust_janus::*;
use rust_janus::error::jsonrpc_error::*;
mod test_utils;
use test_utils::*;

/// Janus Datagram Client Tests - SOCK_DGRAM parity  
/// Tests high-level datagram client functionality, command execution

#[tokio::test]
async fn test_janus_client_initialization() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path.clone(),
        "test".to_string(),
        config,
    ).await.unwrap();
    
    // Test SOCK_DGRAM client properties
    assert_eq!(client.socket_path(), &socket_path);
    assert_eq!(client.channel_id(), "test");
}

#[tokio::test]
async fn test_janus_client_send_command() {
    let _manifest = load_test_manifest();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let mut client = JanusClient::new(
        socket_path,
        "test".to_string(),
        config,
    ).await;
    
    assert!(client.is_ok());
    // Note: Actual send_command tests require a server, covered in integration tests
}

/// Test JSON-RPC 2.0 compliant error handling
/// Validates the architectural enhancement for standardized error codes
#[tokio::test]
async fn test_jsonrpc_error_functionality() {
    // Test error code creation and properties
    let err = JSONRPCError::new(JSONRPCErrorCode::MethodNotFound, Some("Test method not found".to_string()), None);
    
    assert_eq!(err.code, JSONRPCErrorCode::MethodNotFound as i32);
    assert_eq!(err.message, "Test method not found");
    
    // Test error code string representation
    let code_string = JSONRPCErrorCode::MethodNotFound.to_string();
    assert_eq!(code_string, "METHOD_NOT_FOUND");
    
    // Test all standard error codes
    let test_cases = vec![
        (JSONRPCErrorCode::ParseError, "PARSE_ERROR"),
        (JSONRPCErrorCode::InvalidRequest, "INVALID_REQUEST"),
        (JSONRPCErrorCode::MethodNotFound, "METHOD_NOT_FOUND"),
        (JSONRPCErrorCode::InvalidParams, "INVALID_PARAMS"),
        (JSONRPCErrorCode::InternalError, "INTERNAL_ERROR"),
        (JSONRPCErrorCode::ValidationFailed, "VALIDATION_FAILED"),
        (JSONRPCErrorCode::HandlerTimeout, "HANDLER_TIMEOUT"),
        (JSONRPCErrorCode::SecurityViolation, "SECURITY_VIOLATION"),
    ];
    
    for (code, expected) in test_cases {
        assert_eq!(code.to_string(), expected, "Error code {:?} string mismatch", code);
    }
    
    // Test JSON serialization of error
    let json_result = serde_json::to_string(&err);
    assert!(json_result.is_ok(), "Failed to serialize JSONRPCError to JSON");
    
    let json_string = json_result.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_string).unwrap();
    
    assert_eq!(parsed["code"], JSONRPCErrorCode::MethodNotFound as i32);
    assert_eq!(parsed["message"], "Test method not found");
    
    // Test error deserialization
    let deserialized: Result<JSONRPCError, _> = serde_json::from_str(&json_string);
    assert!(deserialized.is_ok(), "Failed to deserialize JSONRPCError from JSON");
    
    let deserialized_err = deserialized.unwrap();
    assert_eq!(deserialized_err.code, err.code);
    assert_eq!(deserialized_err.message, err.message);
}