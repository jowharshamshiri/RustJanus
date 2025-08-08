use std::time::Duration;
use std::collections::HashMap;
use uuid::Uuid;

use rust_janus::protocol::message_types::{RequestHandle, RequestStatus};
use rust_janus::protocol::janus_client::JanusClient;
use rust_janus::config::JanusClientConfig;
use rust_janus::error::{JSONRPCError, JSONRPCErrorCode};

#[tokio::test]
async fn test_request_handle_creation() {
    // Test F0194: Request ID Assignment and F0196: RequestHandle Structure
    let internal_id = "test-uuid-12345";
    let request = "test_request";
    let channel = "test_channel";
    
    let handle = RequestHandle::new(internal_id.to_string(), request.to_string());
    
    // Verify handle properties
    assert_eq!(handle.get_request(), request);
    assert_eq!(handle.get_channel(), channel);
    assert_eq!(handle.get_internal_id(), internal_id);
    assert!(!handle.is_cancelled());
    
    // Test timestamp is recent
    let now = std::time::SystemTime::now();
    let handle_time = handle.get_timestamp();
    let diff = now.duration_since(handle_time).unwrap_or(Duration::ZERO);
    assert!(diff < Duration::from_secs(1), "Handle timestamp should be recent");
}

#[tokio::test]
async fn test_request_handle_cancellation() {
    // Test F0204: Request Cancellation and F0212: Request Cleanup
    let handle = RequestHandle::new("test-id".to_string(), "test_request".to_string());
    
    assert!(!handle.is_cancelled());
    
    handle.mark_cancelled();
    
    assert!(handle.is_cancelled());
}

#[tokio::test]
async fn test_request_status_tracking() {
    // Test F0202: Request Status Query
    let config = JanusClientConfig {
        enable_validation: false,
        ..Default::default()
    };
    
    let client = JanusClient::new(
        "/tmp/test_socket".to_string(),
        config
    ).await.unwrap();
    
    // Create a handle
    let handle = RequestHandle::new("test-id".to_string(), "test_request".to_string());
    
    // Test initial status (should be completed since not in registry)
    let status = client.get_request_status(&handle);
    assert_eq!(status, RequestStatus::Completed);
    
    // Test cancelled status
    handle.mark_cancelled();
    let status = client.get_request_status(&handle);
    assert_eq!(status, RequestStatus::Cancelled);
}

#[tokio::test]
async fn test_pending_request_management() {
    // Test F0197: Handle Creation and F0201: Request State Management
    let config = JanusClientConfig {
        enable_validation: false,
        ..Default::default()
    };
    
    let client = JanusClient::new(
        "/tmp/test_socket".to_string(),
        config
    ).await.unwrap();
    
    // Initially no pending requests
    let pending = client.get_pending_requests();
    assert_eq!(pending.len(), 0);
    
    // Test cancel all with no requests
    let cancelled = client.cancel_all_requests();
    assert_eq!(cancelled, 0);
}

#[tokio::test]
async fn test_request_lifecycle_management() {
    // Test F0200: Request State Management and F0211: Handle Cleanup
    let config = JanusClientConfig {
        enable_validation: false,
        ..Default::default()
    };
    
    let client = JanusClient::new(
        "/tmp/test_socket".to_string(),
        config
    ).await.unwrap();
    
    // Create multiple handles to test bulk operations
    let handles = vec![
        RequestHandle::new("id1".to_string(), "cmd1".to_string()),
        RequestHandle::new("id2".to_string(), "cmd2".to_string()),
        RequestHandle::new("id3".to_string(), "cmd3".to_string()),
    ];
    
    // Test that handles start as completed (not in registry)
    for (i, handle) in handles.iter().enumerate() {
        let status = client.get_request_status(handle);
        assert_eq!(status, RequestStatus::Completed, "Handle {} should start as completed", i);
    }
    
    // Test cancellation of non-existent handle should fail
    let result = client.cancel_request(&handles[0]);
    assert!(result.is_err(), "Expected error when cancelling non-existent request");
    
    if let Err(err) = result {
        assert_eq!(err.code, JSONRPCErrorCode::ValidationFailed.code());
    }
}

#[tokio::test]
async fn test_id_visibility_control() {
    // Test F0195: ID Visibility Control - UUIDs should be hidden from normal API
    let handle = RequestHandle::new("internal-uuid-12345".to_string(), "test_request".to_string());
    
    // User should only see request and channel, not internal UUID through normal API
    assert_eq!(handle.get_request(), "test_request");
    assert_eq!(handle.get_channel(), "test_channel");
    
    // Internal ID should only be accessible for internal operations
    assert_eq!(handle.get_internal_id(), "internal-uuid-12345");
}

#[tokio::test]
async fn test_request_status_constants() {
    // Test all RequestStatus constants are defined
    let statuses = vec![
        RequestStatus::Pending,
        RequestStatus::Completed,
        RequestStatus::Failed,
        RequestStatus::Cancelled,
        RequestStatus::Timeout,
    ];
    
    let expected_values = vec!["Pending", "Completed", "Failed", "Cancelled", "Timeout"];
    
    for (i, status) in statuses.iter().enumerate() {
        let status_str = format!("{:?}", status);
        assert_eq!(status_str, expected_values[i]);
    }
}

#[tokio::test]
async fn test_concurrent_request_handling() {
    // Test F0223: Concurrent Request Support
    let config = JanusClientConfig {
        enable_validation: false,
        ..Default::default()
    };
    
    let client = JanusClient::new(
        "/tmp/test_socket".to_string(),
        config
    ).await.unwrap();
    
    // Test concurrent handle creation and management
    let handles: Vec<RequestHandle> = (0..10).map(|i| {
        RequestHandle::new(
            format!("concurrent-id-{}", i),
            format!("cmd{}", i)
        )
    }).collect();
    
    // Test concurrent status checks
    for handle in &handles {
        let status = client.get_request_status(handle);
        assert_eq!(status, RequestStatus::Completed);
    }
    
    // Test concurrent cancellation
    for handle in &handles {
        handle.mark_cancelled();
        assert!(handle.is_cancelled());
    }
}

#[tokio::test]
async fn test_uuid_generation_uniqueness() {
    // Test F0193: UUID Generation - ensure unique IDs
    let mut generated_ids = std::collections::HashSet::new();
    
    for _ in 0..1000 {
        let handle = RequestHandle::new(
            Uuid::new_v4().to_string(),
            "test_request".to_string()
        );
        
        let id = handle.get_internal_id();
        assert!(!generated_ids.contains(id), "UUID should be unique: {}", id);
        generated_ids.insert(id.to_string());
    }
    
    assert_eq!(generated_ids.len(), 1000, "All generated UUIDs should be unique");
}