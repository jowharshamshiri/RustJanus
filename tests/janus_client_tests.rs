use rust_janus::*;
mod test_utils;
use test_utils::*;

/// Unix Socket API Datagram Client Tests - SOCK_DGRAM parity  
/// Tests high-level datagram client functionality, command execution

#[tokio::test]
async fn test_datagram_client_initialization() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = JanusDatagramClient::new(
        socket_path.clone(),
        "test-channel".to_string(),
        Some(api_spec),
        config,
    );
    
    assert!(client.is_ok());
    let client = client.unwrap();
    
    // Test SOCK_DGRAM client properties
    assert_eq!(client.socket_path(), &socket_path);
    assert_eq!(client.channel_id(), "test-channel");
}

#[tokio::test]
async fn test_datagram_client_send_command() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = JanusDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    );
    
    assert!(client.is_ok());
    // Note: Actual send_command tests require a server, covered in integration tests
}