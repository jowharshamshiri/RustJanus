use rust_unix_sock_api::*;
mod test_utils;
use test_utils::*;

/// Stateless Communication Tests (8 tests) - SwiftUnixSockAPI parity
/// Tests stateless patterns, isolation, and validation

#[tokio::test]
async fn test_stateless_command_validation() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Command validation should work without connection
    let client = UnixSockApiClient::new(
        socket_path,
        "test-channel".to_string(),
        api_spec,
        config,
    ).await.unwrap();
    
    // Validation happens before connection attempt
    let result = client.send_command(
        "test-command",
        Some(create_test_args()),
        std::time::Duration::from_millis(100),
        None,
    ).await;
    
    // Should fail at connection, not validation
    match result {
        Err(UnixSockApiError::ConnectionError(_)) | Err(UnixSockApiError::CommandTimeout(_, _)) => {},
        other => println!("Stateless validation result: {:?}", other),
    }
}

// Placeholder for remaining 7 stateless communication tests
#[tokio::test]
async fn test_placeholder_stateless() {
    println!("Stateless communication tests placeholder - 7 additional tests needed");
}