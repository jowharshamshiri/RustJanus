use std::time::Duration;
use tokio::time::sleep;
use janus::protocol::timeout_manager::{TimeoutManager, TimeoutHandler};

/// Test timeout extension capability (matches Go/TypeScript implementation)
#[tokio::test]
async fn test_timeout_extension() {
    let manager = TimeoutManager::new();
    
    let command_id = "test-extend-command".to_string();
    let timeout_fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let timeout_fired_clone = timeout_fired.clone();
    
    let callback: TimeoutHandler = Box::new(move |_cmd_id, _duration| {
        timeout_fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    
    // Register a timeout for 100ms
    manager.start_timeout(command_id.clone(), Duration::from_millis(100), Some(callback))
        .await
        .expect("Failed to start timeout");
    
    // Wait 50ms, then extend by 100ms
    sleep(Duration::from_millis(50)).await;
    let extended = manager.extend_timeout(&command_id, Duration::from_millis(100)).await;
    
    assert!(extended, "Expected timeout extension to succeed");
    
    // Wait another 100ms (should not fire yet since we extended)
    sleep(Duration::from_millis(100)).await;
    
    assert!(!timeout_fired.load(std::sync::atomic::Ordering::SeqCst), 
           "Callback should not have fired yet after extension");
    
    // Wait for the extended timeout to fire
    sleep(Duration::from_millis(100)).await;
    
    assert!(timeout_fired.load(std::sync::atomic::Ordering::SeqCst), 
           "Callback should have fired after extended timeout");
    
    // Test extending non-existent timeout
    let non_existent_extended = manager.extend_timeout("non-existent", Duration::from_millis(100)).await;
    assert!(!non_existent_extended, "Expected extension of non-existent timeout to fail");
}

/// Test error-handled registration (matches TypeScript error-handled registration pattern)
#[tokio::test]
async fn test_error_handled_registration() {
    let manager = TimeoutManager::new();
    
    let command_id = "test-error-handled".to_string();
    let timeout_fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let timeout_fired_clone = timeout_fired.clone();
    
    let callback: TimeoutHandler = Box::new(move |_cmd_id, _duration| {
        timeout_fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    
    // Register timeout with error callback
    manager.start_timeout_with_error_handler(
        command_id.clone(), 
        Duration::from_millis(50), 
        Some(callback), 
        None
    ).await.expect("Failed to start timeout with error handler");
    
    assert_eq!(manager.active_timeout_count().await, 1, "Expected 1 active timeout");
    
    // Wait for timeout to fire
    sleep(Duration::from_millis(100)).await;
    
    assert!(timeout_fired.load(std::sync::atomic::Ordering::SeqCst), 
           "Main callback should have been called");
    
    assert_eq!(manager.active_timeout_count().await, 0, 
              "Expected 0 active timeouts after firing");
}

/// Test bilateral timeout management (matches Go/TypeScript bilateral timeout implementation)
#[tokio::test]
async fn test_bilateral_timeout_management() {
    let manager = TimeoutManager::new();
    
    let bilateral_timeout_fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let bilateral_timeout_fired_clone = bilateral_timeout_fired.clone();
    
    let bilateral_callback: TimeoutHandler = Box::new(move |_cmd_id, _duration| {
        bilateral_timeout_fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    
    // Register bilateral timeout
    let base_command_id = "test-bilateral";
    
    manager.start_bilateral_timeout(base_command_id, Duration::from_millis(100), Some(bilateral_callback))
        .await
        .expect("Failed to start bilateral timeout");
    
    // Should have 2 active timeouts (request and response)
    assert_eq!(manager.active_timeout_count().await, 2, 
              "Expected 2 active timeouts for bilateral");
    
    // Cancel bilateral timeout
    let cancelled_count = manager.cancel_bilateral_timeout(base_command_id).await;
    
    assert_eq!(cancelled_count, 2, "Expected to cancel 2 timeouts");
    assert_eq!(manager.active_timeout_count().await, 0, 
              "Expected 0 active timeouts after cancellation");
    
    // Wait to ensure callback doesn't fire
    sleep(Duration::from_millis(150)).await;
    
    assert!(!bilateral_timeout_fired.load(std::sync::atomic::Ordering::SeqCst), 
           "Bilateral callback should not have fired after cancellation");
}

/// Test bilateral timeout expiration
#[tokio::test]
async fn test_bilateral_timeout_expiration() {
    let manager = TimeoutManager::new();
    
    let bilateral_timeout_fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let bilateral_timeout_fired_clone = bilateral_timeout_fired.clone();
    
    let bilateral_callback: TimeoutHandler = Box::new(move |_cmd_id, _duration| {
        bilateral_timeout_fired_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    
    // Register bilateral timeout with short duration
    let base_command_id = "test-bilateral-expire";
    
    manager.start_bilateral_timeout(base_command_id, Duration::from_millis(50), Some(bilateral_callback))
        .await
        .expect("Failed to start bilateral timeout");
    
    // Wait for timeout to expire
    sleep(Duration::from_millis(100)).await;
    
    assert!(bilateral_timeout_fired.load(std::sync::atomic::Ordering::SeqCst), 
           "Bilateral callback should have fired after timeout");
    
    assert_eq!(manager.active_timeout_count().await, 0, 
              "Expected 0 active timeouts after expiration");
}

/// Test timeout statistics accuracy (matches Go/TypeScript statistics implementation)
#[tokio::test]
async fn test_timeout_statistics_accuracy() {
    let manager = TimeoutManager::new();
    
    // Register multiple timeouts with different durations
    let timeout1 = Duration::from_millis(100);
    let timeout2 = Duration::from_millis(200);
    let timeout3 = Duration::from_millis(50);
    
    manager.start_timeout("cmd1".to_string(), timeout1, None)
        .await.expect("Failed to start timeout 1");
    manager.start_timeout("cmd2".to_string(), timeout2, None)
        .await.expect("Failed to start timeout 2");
    manager.start_timeout("cmd3".to_string(), timeout3, None)
        .await.expect("Failed to start timeout 3");
    
    let stats = manager.get_timeout_statistics().await;
    
    assert_eq!(manager.active_timeout_count().await, 3, "Expected 3 active timeouts");
    assert_eq!(stats.total_registered, 3, "Expected 3 total registered");
    assert_eq!(stats.total_cancelled, 0, "Expected 0 total cancelled");
    assert_eq!(stats.total_expired, 0, "Expected 0 total expired");
    
    // Cancel one timeout
    let cancelled = manager.cancel_timeout("cmd2").await;
    assert!(cancelled, "Expected timeout cancellation to succeed");
    
    // Check updated statistics
    let stats_after_cancel = manager.get_timeout_statistics().await;
    assert_eq!(stats_after_cancel.total_cancelled, 1, 
              "Expected 1 total cancelled after cancellation");
    assert_eq!(manager.active_timeout_count().await, 2, 
              "Expected 2 active timeouts after cancellation");
    
    // Wait for remaining timeouts to expire
    sleep(Duration::from_millis(250)).await;
    
    let final_stats = manager.get_timeout_statistics().await;
    assert_eq!(final_stats.total_expired, 2, "Expected 2 total expired");
    assert_eq!(manager.active_timeout_count().await, 0, 
              "Expected 0 active timeouts after expiration");
}

/// Test timeout manager concurrency (ensures thread safety of enhanced timeout management)
#[tokio::test]
async fn test_timeout_manager_concurrency() {
    let manager = std::sync::Arc::new(TimeoutManager::new());
    
    // Launch multiple tasks registering timeouts concurrently
    let num_tasks = 10;
    let timeouts_per_task = 5;
    let mut handles = Vec::new();
    
    for task_id in 0..num_tasks {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            for timeout_id in 0..timeouts_per_task {
                let command_id = format!("concurrent-{}-{}", task_id, timeout_id);
                manager_clone.start_timeout(command_id, Duration::from_millis(100), None)
                    .await
                    .expect("Failed to start concurrent timeout");
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete registration
    for handle in handles {
        handle.await.expect("Task failed");
    }
    
    // Give tasks time to register timeouts
    sleep(Duration::from_millis(50)).await;
    
    let stats = manager.get_timeout_statistics().await;
    let expected_timeouts = num_tasks * timeouts_per_task;
    
    assert_eq!(stats.total_registered, expected_timeouts as u64, 
              "Expected {} total registered timeouts", expected_timeouts);
    assert_eq!(manager.active_timeout_count().await, expected_timeouts, 
              "Expected {} active timeouts", expected_timeouts);
    
    // Test concurrent cancellations
    let mut cancel_handles = Vec::new();
    
    for task_id in 0..num_tasks {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            for timeout_id in 0..(timeouts_per_task / 2) {
                let command_id = format!("concurrent-{}-{}", task_id, timeout_id);
                manager_clone.cancel_timeout(&command_id).await;
            }
        });
        cancel_handles.push(handle);
    }
    
    // Wait for cancellations to complete
    for handle in cancel_handles {
        handle.await.expect("Cancel task failed");
    }
    
    // Give cancellations time to complete
    sleep(Duration::from_millis(50)).await;
    
    // Wait for remaining timeouts to expire
    sleep(Duration::from_millis(200)).await;
    
    let final_stats = manager.get_timeout_statistics().await;
    
    // Should have some cancelled and some expired
    let total_processed = final_stats.total_cancelled + final_stats.total_expired;
    assert_eq!(total_processed, expected_timeouts as u64, 
              "Expected total processed (cancelled + expired) to be {}", expected_timeouts);
    
    assert_eq!(manager.active_timeout_count().await, 0, 
              "Expected 0 active timeouts after test completion");
}

/// Test timeout extension boundary conditions
#[tokio::test]
async fn test_timeout_extension_boundary_conditions() {
    let manager = TimeoutManager::new();
    
    // Test extending with zero duration
    manager.start_timeout("test-zero-extend".to_string(), Duration::from_millis(100), None)
        .await.expect("Failed to start timeout");
    let extended = manager.extend_timeout("test-zero-extend", Duration::from_millis(0)).await;
    assert!(extended, "Expected zero-duration extension to succeed");
    
    // Test extending with very small duration
    let extended = manager.extend_timeout("test-zero-extend", Duration::from_nanos(1)).await;
    assert!(extended, "Expected tiny extension to succeed");
    
    // Test extending already expired timeout
    manager.start_timeout("test-quick-expire".to_string(), Duration::from_millis(1), None)
        .await.expect("Failed to start quick timeout");
    sleep(Duration::from_millis(10)).await; // Let it expire
    let extended = manager.extend_timeout("test-quick-expire", Duration::from_millis(100)).await;
    assert!(!extended, "Expected extension of expired timeout to fail");
}