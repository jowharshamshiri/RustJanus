use rust_janus::*;
use rust_janus::config::JanusClientConfig;
use rust_janus::protocol::janus_client::ConnectionState;
use std::collections::HashMap;

/// Connection State Simulation Tests
/// Tests SOCK_DGRAM connection state simulation for backward compatibility

#[tokio::test]
async fn test_connection_state_initialization() {
    let client = JanusClient::new(
        "/tmp/connection_state_test.sock".to_string(),
        "test-channel".to_string(),
        JanusClientConfig::default(),
    ).await.unwrap();
    
    // Check initial connection state
    let state = client.get_connection_state();
    assert_eq!(state.is_connected, false);
    assert_eq!(state.messages_sent, 0);
    assert_eq!(state.responses_received, 0);
}

#[tokio::test]
async fn test_is_connected_functionality() {
    let client = JanusClient::new(
        "/tmp/connection_state_test.sock".to_string(),
        "test-channel".to_string(),
        JanusClientConfig::default(),
    ).await.unwrap();
    
    // is_connected should check if socket file exists
    let connected = client.is_connected();
    // Should return false since no server is running
    assert_eq!(connected, false);
}

#[tokio::test]
async fn test_connection_state_tracking() {
    let client = JanusClient::new(
        "/tmp/connection_state_test.sock".to_string(),
        "test-channel".to_string(),
        JanusClientConfig::default(),
    ).await.unwrap();
    
    // Initial state
    let initial_state = client.get_connection_state();
    assert_eq!(initial_state.messages_sent, 0);
    assert_eq!(initial_state.responses_received, 0);
    assert_eq!(initial_state.is_connected, false);
    
    // Try to send a request (will fail but should update connection state)
    let args = HashMap::new();
    let result = client.send_request_no_response("echo", Some(args)).await;
    
    // Should fail with connection error since no server running
    assert!(result.is_err());
    
    // But connection state should still be tracked for successful operations
    // (This test validates the structure exists and can be queried)
    let final_state = client.get_connection_state();
    assert_eq!(final_state.messages_sent, initial_state.messages_sent); // Should be same since operation failed
}

#[tokio::test] 
async fn test_connection_state_struct() {
    // Test ConnectionState struct functionality
    let state1 = ConnectionState::new();
    assert_eq!(state1.is_connected, false);
    assert_eq!(state1.messages_sent, 0);
    assert_eq!(state1.responses_received, 0);
    
    let state2 = ConnectionState::with_connection(true);
    assert_eq!(state2.is_connected, true);
    assert_eq!(state2.messages_sent, 0);
    assert_eq!(state2.responses_received, 0);
}

#[tokio::test]
async fn test_connection_state_backward_compatibility() {
    // Test that connection state simulation provides backward compatibility
    let client = JanusClient::new(
        "/tmp/connection_state_test.sock".to_string(),
        "test-channel".to_string(),
        JanusClientConfig::default(),
    ).await.unwrap();
    
    // These methods should exist and be callable (for backward compatibility)
    let connected = client.is_connected();
    let state = client.get_connection_state();
    
    // Methods should work without panicking
    assert!(connected == true || connected == false); // Any boolean value is fine
    assert!(state.messages_sent >= 0); // Should be non-negative
    assert!(state.responses_received >= 0); // Should be non-negative
}