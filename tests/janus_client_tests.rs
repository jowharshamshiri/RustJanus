use rust_janus::*;
mod test_utils;
use test_utils::*;

/// Janus Datagram Client Tests - SOCK_DGRAM parity  
/// Tests high-level datagram client functionality, command execution

#[tokio::test]
async fn test_janus_client_initialization() {
    let api_spec = load_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = JanusClient::new(
        socket_path.clone(),
        "test".to_string(),
        Some(api_spec),
        config,
    ).await;
    
    assert!(client.is_ok());
    let client = client.unwrap();
    
    // Test SOCK_DGRAM client properties
    assert_eq!(client.socket_path(), &socket_path);
    assert_eq!(client.channel_id(), "test");
}

#[tokio::test]
async fn test_janus_client_send_command() {
    let api_spec = load_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = JanusClient::new(
        socket_path,
        "test".to_string(),
        Some(api_spec),
        config,
    ).await;
    
    assert!(client.is_ok());
    // Note: Actual send_command tests require a server, covered in integration tests
}