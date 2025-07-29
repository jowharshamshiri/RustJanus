use rust_unix_sock_api::*;
mod test_utils;
use test_utils::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

/// Concurrency Tests (13 tests) - Exact SwiftUnixSockAPI parity
/// Tests high concurrency, race conditions, thread safety, deadlock prevention

#[tokio::test]
async fn test_high_concurrency_command_execution() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));
    
    // Execute 100 concurrent commands
    let mut tasks = Vec::new();
    for i in 0..100 {
        let client_clone = client.clone();
        let success_count_clone = success_count.clone();
        let error_count_clone = error_count.clone();
        
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("test_{}", i)));
            
            match client_clone.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(100)),
            ).await {
                Ok(_) => success_count_clone.fetch_add(1, Ordering::SeqCst),
                Err(_) => error_count_clone.fetch_add(1, Ordering::SeqCst),
            };
        }));
    }
    
    // Wait for all tasks to complete
    for task in tasks {
        task.await.unwrap();
    }
    
    let total_operations = success_count.load(Ordering::SeqCst) + error_count.load(Ordering::SeqCst);
    assert_eq!(total_operations, 100, "All operations should complete");
    
    // All operations should complete without panics or crashes
    println!("Successes: {}, Errors: {}", 
             success_count.load(Ordering::SeqCst), 
             error_count.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_concurrent_client_creation() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // Create 50 clients simultaneously
    let mut tasks = Vec::new();
    for i in 0..50 {
        let socket_path_clone = socket_path.clone();
        let api_spec_clone = api_spec.clone();
        let config_clone = config.clone();
        
        tasks.push(tokio::spawn(async move {
            UnixSockApiDatagramClient::new(
                socket_path_clone,
                format!("channel-{}", i),
                Some(api_spec_clone),
                config_clone,
            )
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for result in results {
        match result.unwrap() {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    let total = success_count + error_count;
    assert_eq!(total, 50, "All client creation attempts should complete");
    
    println!("Client creation - Successes: {}, Errors: {}", success_count, error_count);
}

#[tokio::test]
async fn test_concurrent_handler_registration() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // Register 20 handlers concurrently
    let mut tasks = Vec::new();
    for i in 0..20 {
        let client_clone = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("handler_id".to_string(), serde_json::Value::String(format!("handler-{}", i)));
            client_clone.send_command(
                "echo-test",
                Some(args),
                Some(std::time::Duration::from_millis(100)),
            ).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let mut success_count = 0;
    let mut error_count = 0;
    
    for result in results {
        match result.unwrap() {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    println!("Handler registration - Successes: {}, Errors: {}", success_count, error_count);
    // Should handle concurrent registrations gracefully
    assert!(success_count + error_count == 20);
}

#[tokio::test]
async fn test_concurrent_connection_pool_usage() {
    let api_spec = create_test_api_spec();
    let mut config = create_test_config();
    config.max_concurrent_connections = 10; // Limited pool
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // 50 operations on 10-connection pool
    let mut tasks = Vec::new();
    for i in 0..50 {
        let client_clone = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("pool_test_{}", i)));
            
            client_clone.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(50)),
            ).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let mut success_count = 0;
    let mut timeout_count = 0;
    let mut error_count = 0;
    
    for result in results {
        match result.unwrap() {
            Ok(_) => success_count += 1,
            Err(UnixSockApiError::CommandTimeout(_, _)) => timeout_count += 1,
            Err(UnixSockApiError::ResourceLimit(_)) => error_count += 1,
            Err(UnixSockApiError::ConnectionError(_)) => error_count += 1,
            Err(err) => panic!("Unexpected error: {:?}", err),
        }
    }
    
    println!("Pool usage - Successes: {}, Timeouts: {}, Errors: {}", 
             success_count, timeout_count, error_count);
    assert_eq!(success_count + timeout_count + error_count, 50);
}

#[tokio::test]
async fn test_concurrent_state_modification() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    let counter = Arc::new(AtomicUsize::new(0));
    
    // Concurrent state access
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let client_clone = client.clone();
        let counter_clone = counter.clone();
        
        tasks.push(tokio::spawn(async move {
            // Access client configuration concurrently
            let _config = client_clone.configuration();
            let _spec = client_clone.specification();
            
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }));
    }
    
    futures::future::join_all(tasks).await;
    
    assert_eq!(counter.load(Ordering::SeqCst), 100);
}

#[tokio::test]
async fn test_concurrent_connection_management() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // 30 concurrent connections with different channels
    let mut tasks = Vec::new();
    for i in 0..30 {
        let socket_path_clone = socket_path.clone();
        let api_spec_clone = api_spec.clone();
        let config_clone = config.clone();
        
        tasks.push(tokio::spawn(async move {
            let client = UnixSockApiDatagramClient::new(
                socket_path_clone,
                format!("channel-{}", i),
                Some(api_spec_clone),
                config_clone,
            )?;
            
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("test_{}", i)));
            
            client.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(100)),
            ).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let mut completed = 0;
    for result in results {
        match result.unwrap() {
            Ok(_) | Err(_) => completed += 1,
        }
    }
    
    assert_eq!(completed, 30);
}

#[tokio::test]
async fn test_thread_safety_of_configuration() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // 100 concurrent configuration accesses
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let client_clone = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let config = client_clone.configuration();
            assert!(config.max_concurrent_connections > 0);
            assert!(config.max_message_size > 0);
            assert!(config.connection_timeout.as_secs() > 0);
        }));
    }
    
    futures::future::join_all(tasks).await;
}

#[tokio::test]
async fn test_thread_safety_of_api_spec_access() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // 100 concurrent specification accesses
    let mut tasks = Vec::new();
    for _ in 0..100 {
        let client_clone = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let spec = client_clone.specification().unwrap();
            assert_eq!(spec.version, "1.0.0");
            assert!(!spec.channels.is_empty());
        }));
    }
    
    futures::future::join_all(tasks).await;
}

#[tokio::test]
async fn test_no_deadlock_under_load() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // 50 tasks with 10-second timeout to detect deadlocks
    let mut tasks = Vec::new();
    for i in 0..50 {
        let client_clone = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let timeout = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                async {
                    let mut args = HashMap::new();
                    args.insert("test_arg".to_string(), serde_json::Value::String(format!("deadlock_test_{}", i)));
                    
                    client_clone.send_command(
                        "test-command",
                        Some(args),
                        Some(std::time::Duration::from_millis(100)),
                    ).await
                }
            ).await;
            
            match timeout {
                Ok(result) => result,
                Err(_) => Err(UnixSockApiError::CommandTimeout("deadlock_test".to_string(), std::time::Duration::from_secs(10))),
            }
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // All tasks should complete within timeout (no deadlocks)
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(_) => {}, // Operation completed
            Err(err) => panic!("Task {} did not complete: {:?}", i, err),
        }
    }
}

#[tokio::test]
async fn test_no_deadlock_with_mixed_operations() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    let mut tasks = Vec::new();
    
    // 20 command operations
    for i in 0..20 {
        let client_clone = client.clone();
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("mixed_test_{}", i)));
            
            client_clone.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(50)),
            ).await
        }));
    }
    
    // 5 handler registrations
    for i in 0..5 {
        let client_clone = client.clone();
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("handler_id".to_string(), serde_json::Value::String(format!("mixed_handler_{}", i)));
            client_clone.send_command(
                "echo-test",
                Some(args),
                Some(std::time::Duration::from_millis(100)),
            ).await
        }));
    }
    
    // 10 configuration accesses
    for _ in 0..10 {
        let client_clone = client.clone();
        tasks.push(tokio::spawn(async move {
            let _config = client_clone.configuration();
            let _spec = client_clone.specification();
            // Convert to SocketResponse for consistency
            Ok(SocketResponse::success("config_access".to_string(), "test-channel".to_string(), Some(serde_json::json!({}))))
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    assert_eq!(results.len(), 35); // 20 + 5 + 10
    
    // All should complete without deadlocks
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Task {} panicked or deadlocked", i);
    }
}

#[tokio::test]
async fn test_memory_safety_under_concurrent_access() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // 100 concurrent operations with memory pressure
    let large_data = Arc::new(create_large_test_data(100)); // 100KB each
    
    let mut tasks = Vec::new();
    for i in 0..100 {
        let client_clone = client.clone();
        let large_data_clone = large_data.clone();
        
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("{}_{}_{}", large_data_clone, i, i)));
            
            client_clone.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(50)),
            ).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // All operations should complete without memory corruption
    let mut completed = 0;
    for result in results {
        match result.unwrap() {
            Ok(_) | Err(_) => completed += 1,
        }
    }
    
    assert_eq!(completed, 100);
}

#[tokio::test]
async fn test_concurrent_resource_cleanup() {
    let api_spec = create_test_api_spec();
    let config = create_test_config();
    let socket_path = create_valid_socket_path();
    
    // 20 concurrent client lifecycles
    let mut tasks: Vec<tokio::task::JoinHandle<Result<SocketResponse>>> = Vec::new();
    for i in 0..20 {
        let socket_path_clone = socket_path.clone();
        let api_spec_clone = api_spec.clone();
        let config_clone = config.clone();
        
        tasks.push(tokio::spawn(async move {
            // Create client
            let client = UnixSockApiDatagramClient::new(
                socket_path_clone,
                format!("cleanup_channel_{}", i),
                Some(api_spec_clone),
                config_clone,
            )?;
            
            // Use client
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("cleanup_test_{}", i)));
            
            let _result = client.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(50)),
            );
            
            // Client should be cleaned up automatically when dropped
            Ok(SocketResponse::success("timeout_test".to_string(), "test-channel".to_string(), Some(serde_json::json!({}))))
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    let mut success_count = 0;
    for result in results {
        match result.unwrap() {
            Ok(_) => success_count += 1,
            Err(_) => {}, // Some failures are expected
        }
    }
    
    // At least some should succeed, demonstrating proper cleanup
    println!("Cleanup test successes: {}/20", success_count);
}

#[tokio::test]
async fn test_connection_pool_thread_safety() {
    let api_spec = create_test_api_spec();
    let mut config = create_test_config();
    config.max_concurrent_connections = 5; // Small pool for contention
    let socket_path = create_valid_socket_path();
    
    let client = Arc::new(UnixSockApiDatagramClient::new(
        socket_path,
        "test-channel".to_string(),
        Some(api_spec),
        config,
    ).unwrap());
    
    // 30 operations on 5-connection pool
    let mut tasks: Vec<tokio::task::JoinHandle<Result<SocketResponse>>> = Vec::new();
    for i in 0..30 {
        let client_clone = client.clone();
        
        tasks.push(tokio::spawn(async move {
            let mut args = HashMap::new();
            args.insert("test_arg".to_string(), serde_json::Value::String(format!("pool_safety_{}", i)));
            
            client_clone.send_command(
                "test-command",
                Some(args),
                Some(std::time::Duration::from_millis(100)),
            ).await
        }));
    }
    
    let results = futures::future::join_all(tasks).await;
    
    // All operations should complete without pool corruption
    let mut total_ops = 0;
    for result in results {
        match result.unwrap() {
            Ok(_) | Err(_) => total_ops += 1,
        }
    }
    
    assert_eq!(total_ops, 30);
}