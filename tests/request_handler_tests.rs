use rust_janus::error::{JSONRPCError, JSONRPCErrorCode};
use rust_janus::protocol::request_handler::*;
use rust_janus::protocol::message_types::JanusRequest;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::time::sleep;

/**
 * Comprehensive RequestHandler Tests for Rust Janus Implementation
 * Tests all direct value response handlers, async patterns, error handling, and JSON-RPC error mapping
 * Matches Go, TypeScript, and Swift test coverage for cross-platform parity
 */

// Helper function to create test requests
fn create_test_request(
    id: Option<String>,
    channel_id: Option<String>,
    request: Option<String>,
    args: std::collections::HashMap<String, serde_json::Value>,
    reply_to: Option<String>,
) -> JanusRequest {
    JanusRequest {
        id: id.unwrap_or_else(|| "test-id".to_string()),
        channelId: channel_id.unwrap_or_else(|| "test-channel".to_string()),
        request: request.unwrap_or_else(|| "test-request".to_string()),
        args: Some(args),
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
        reply_to,
        timeout: None,
    }
}

#[tokio::test]
async fn test_bool_handler() {
    // Test boolean handler returning true
    let handler = bool_handler(|_cmd| Ok(true));
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value, true, "Boolean handler should return true");
        }
        HandlerResult::Error(error) => {
            panic!("Boolean handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_string_handler() {
    // Test string handler returning test response
    let handler = string_handler(|_cmd| Ok("test response".to_string()));
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value, "test response", "String handler should return 'test response'");
        }
        HandlerResult::Error(error) => {
            panic!("String handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_int_handler() {
    // Test integer handler returning 42
    let handler = int_handler(|_cmd| Ok(42));
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value, 42, "Int handler should return 42");
        }
        HandlerResult::Error(error) => {
            panic!("Int handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_float_handler() {
    // Test float handler returning 3.14
    let handler = float_handler(|_cmd| Ok(3.14));
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert!((value - 3.14).abs() < 0.001, "Float handler should return 3.14");
        }
        HandlerResult::Error(error) => {
            panic!("Float handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_array_handler() {
    // Test array handler returning test array
    let test_array = vec!["item1".to_string(), "item2".to_string(), "item3".to_string()];
    let expected_array = test_array.clone();
    
    let handler = array_handler(move |_cmd| Ok(test_array.clone()));
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value, expected_array, "Array handler should return test array");
        }
        HandlerResult::Error(error) => {
            panic!("Array handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_object_handler() {
    // Test custom object handler
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        id: i64,
        name: String,
    }
    
    let test_user = TestUser {
        id: 123,
        name: "Test User".to_string(),
    };
    let expected_user = test_user.clone();
    
    let handler = object_handler(move |_cmd| Ok(test_user.clone()));
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value, expected_user, "Object handler should return test user");
        }
        HandlerResult::Error(error) => {
            panic!("Object handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_async_bool_handler() {
    // Test async boolean handler with timing verification
    let handler = async_bool_handler(|_cmd| async move {
        sleep(Duration::from_millis(10)).await; // 10ms delay
        Ok(true)
    });
    
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    let start_time = Instant::now();
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            let duration = start_time.elapsed();
            assert!(duration >= Duration::from_millis(10), "Async execution should take at least 10ms");
            assert_eq!(value, true, "Async boolean handler should return true");
        }
        HandlerResult::Error(error) => {
            panic!("Async boolean handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_async_string_handler() {
    // Test async string handler with timing verification
    let handler = async_string_handler(|_cmd| async move {
        sleep(Duration::from_millis(10)).await; // 10ms delay
        Ok("async response".to_string())
    });
    
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    let start_time = Instant::now();
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            let duration = start_time.elapsed();
            assert!(duration >= Duration::from_millis(10), "Async execution should take at least 10ms");
            assert_eq!(value, "async response", "Async string handler should return 'async response'");
        }
        HandlerResult::Error(error) => {
            panic!("Async string handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_async_custom_handler() {
    // Test async custom handler
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ApiResponse {
        success: bool,
        message: String,
        data: std::collections::HashMap<String, i64>,
    }
    
    let mut data = std::collections::HashMap::new();
    data.insert("userId".to_string(), 456);
    
    let test_response = ApiResponse {
        success: true,
        message: "Operation completed".to_string(),
        data,
    };
    let expected_response = test_response.clone();
    
    let handler = async_custom_handler(move |_cmd| {
        let response = test_response.clone();
        async move {
            sleep(Duration::from_millis(5)).await; // 5ms delay
            Ok(response)
        }
    });
    
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value, expected_response, "Async custom handler should return test response");
        }
        HandlerResult::Error(error) => {
            panic!("Async custom handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_sync_handler_error_handling() {
    // Test synchronous handler error handling
    let handler = string_handler(|_cmd| {
        Err("sync handler error".to_string().into())
    });
    
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(_) => {
            panic!("Handler should return error, not success");
        }
        HandlerResult::Error(error) => {
            assert_eq!(error.code, JSONRPCErrorCode::InternalError as i32, "Error should be internal error");
            assert!(error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()).is_some(), "Error should have details");
        }
    }
}

#[tokio::test]
async fn test_async_handler_error_handling() {
    // Test asynchronous handler error handling
    let handler = async_string_handler(|_cmd| async move {
        Err("async handler error".to_string().into())
    });
    
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(_) => {
            panic!("Async handler should return error, not success");
        }
        HandlerResult::Error(error) => {
            assert_eq!(error.code, JSONRPCErrorCode::InternalError as i32, "Error should be internal error");
            assert!(error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()).is_some(), "Error should have details");
        }
    }
}

#[tokio::test]
async fn test_jsonrpc_error_handling() {
    // Test JSON-RPC error handling using a sync handler that returns the error directly
    let handler = SyncHandler::new(|_cmd| {
        let jsonrpc_error = JSONRPCError::new(
            JSONRPCErrorCode::InvalidParams,
            Some("Invalid parameters provided".to_string())
        );
        HandlerResult::<String>::error(jsonrpc_error)
    });
    
    let request = create_test_request(None, None, None, std::collections::HashMap::new(), None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(_) => {
            panic!("Handler should return JSON-RPC error, not success");
        }
        HandlerResult::Error(error) => {
            assert_eq!(error.code, JSONRPCErrorCode::InvalidParams as i32, "Error code should be invalid params");
            assert_eq!(error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()).as_deref(), Some("Invalid parameters provided"), "Should preserve custom error message");
        }
    }
}

#[tokio::test]
async fn test_handler_registry() {
    let registry = HandlerRegistry::new(10);
    
    // Test handler registration
    let handler = string_handler(|_cmd| Ok("registry test".to_string()));
    
    registry.register_handler("test-request".to_string(), handler)
        .await
        .expect("Handler registration should not fail");
    
    // Test handler existence
    let has_handler = registry.has_handler("test-request").await;
    assert!(has_handler, "Registry should have registered handler");
    
    // Test handler execution
    let request = create_test_request(
        None, None, Some("test-request".to_string()), 
        std::collections::HashMap::new(), None
    );
    
    match registry.execute_handler("test-request", &request).await {
        Ok(value) => {
            if let Some(string_value) = value.as_str() {
                assert_eq!(string_value, "registry test", "Handler should return expected value");
            } else {
                panic!("Result should be string type");
            }
        }
        Err(error) => {
            panic!("Handler execution should not fail: {:?}", error);
        }
    }
    
    // Test handler count
    let count = registry.handler_count().await;
    assert_eq!(count, 1, "Registry should have 1 handler");
    
    // Test handler unregistration
    let removed = registry.unregister_handler("test-request").await;
    assert!(removed, "Handler should be successfully unregistered");
    
    let has_handler_after_removal = registry.has_handler("test-request").await;
    assert!(!has_handler_after_removal, "Registry should not have handler after removal");
}

#[tokio::test]
async fn test_handler_registry_limits() {
    let registry = HandlerRegistry::new(2);
    
    // Register maximum number of handlers
    let handler1 = string_handler(|_cmd| Ok("handler1".to_string()));
    let handler2 = string_handler(|_cmd| Ok("handler2".to_string()));
    let handler3 = string_handler(|_cmd| Ok("handler3".to_string()));
    
    registry.register_handler("cmd1".to_string(), handler1)
        .await
        .expect("First registration should succeed");
    
    registry.register_handler("cmd2".to_string(), handler2)
        .await
        .expect("Second registration should succeed");
    
    // Third registration should fail
    match registry.register_handler("cmd3".to_string(), handler3).await {
        Ok(_) => panic!("Third registration should fail due to limit"),
        Err(_) => {
            // Expected error - limit exceeded
        }
    }
    
    let count = registry.handler_count().await;
    assert_eq!(count, 2, "Registry should have exactly 2 handlers");
}

#[tokio::test]
async fn test_handler_registry_not_found() {
    let registry = HandlerRegistry::new(10);
    
    let request = create_test_request(
        None, None, Some("nonexistent-request".to_string()),
        std::collections::HashMap::new(), None
    );
    
    match registry.execute_handler("nonexistent-request", &request).await {
        Ok(_) => panic!("Execution should fail for nonexistent handler"),
        Err(error) => {
            assert_eq!(error.code, JSONRPCErrorCode::MethodNotFound as i32, "Error should be method not found");
            assert!(error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()).unwrap_or_default().contains("Request not found"), "Error should mention request not found");
        }
    }
}

#[tokio::test]
async fn test_handler_argument_access() {
    // Test handler can access and process request arguments
    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct ProcessedData {
        processed_name: String,
        processed_age: i64,
        original_request: String,
    }
    
    let handler = object_handler(|request| {
        let args = request.args.as_ref().ok_or("No arguments provided")?;
        
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing or invalid name argument")?;
        
        let age = args.get("age")
            .and_then(|v| v.as_f64())
            .ok_or("Missing or invalid age argument")? as i64;
        
        let processed_data = ProcessedData {
            processed_name: format!("Hello, {}", name),
            processed_age: age + 1,
            original_request: request.request.clone(),
        };
        
        Ok(processed_data)
    });
    
    let mut args = std::collections::HashMap::new();
    args.insert("name".to_string(), serde_json::Value::String("John".to_string()));
    args.insert("age".to_string(), serde_json::Value::Number(serde_json::Number::from(25)));
    
    let request = create_test_request(
        None, None, Some("process-user".to_string()),
        args, None
    );
    
    match handler.handle(&request).await {
        HandlerResult::Success(value) => {
            assert_eq!(value.processed_name, "Hello, John", "Name should be processed correctly");
            assert_eq!(value.processed_age, 26, "Age should be incremented by 1");
            assert_eq!(value.original_request, "process-user", "Original request should be preserved");
        }
        HandlerResult::Error(error) => {
            panic!("Handler should not return error: {:?}", error);
        }
    }
}

#[tokio::test]
async fn test_handler_argument_validation() {
    // Test handler validates required arguments
    let handler = string_handler(|request| {
        let args = request.args.as_ref().ok_or("No arguments provided")?;
        
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Missing or invalid name argument")?;
        
        let _age = args.get("age")
            .and_then(|v| v.as_f64())
            .ok_or("Missing or invalid age argument")?;
        
        Ok(format!("Hello, {}", name))
    });
    
    // Test with missing age argument
    let mut args = std::collections::HashMap::new();
    args.insert("name".to_string(), serde_json::Value::String("John".to_string()));
    
    let request = create_test_request(None, None, None, args, None);
    
    match handler.handle(&request).await {
        HandlerResult::Success(_) => {
            panic!("Handler should return error for missing arguments");
        }
        HandlerResult::Error(error) => {
            assert_eq!(error.code, JSONRPCErrorCode::InternalError as i32, "Error should be internal error");
            assert!(error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()).unwrap_or_default().contains("Missing or invalid age argument"), "Error should mention missing age argument");
        }
    }
}

#[tokio::test]
async fn test_handler_result_utilities() {
    // Test HandlerResult::success
    let success_result = HandlerResult::success("test value".to_string());
    match success_result {
        HandlerResult::Success(value) => {
            assert_eq!(value, "test value", "HandlerResult should contain test value");
        }
        HandlerResult::Error(_) => {
            panic!("HandlerResult should be success, not error");
        }
    }
    
    // Test HandlerResult::error
    let error = JSONRPCError::new(JSONRPCErrorCode::InternalError, Some("Test error".to_string()));
    let error_result: HandlerResult<String> = HandlerResult::error(error.clone());
    match error_result {
        HandlerResult::Success(_) => {
            panic!("HandlerResult should be error, not success");
        }
        HandlerResult::Error(result_error) => {
            assert_eq!(result_error.code, error.code, "HandlerResult should contain error");
            assert_eq!(result_error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()), error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()), "Error should have correct details");
        }
    }
    
    // Test HandlerResult::from_result success case
    let success_result: Result<String, Box<dyn std::error::Error + Send + Sync>> = Ok("success value".to_string());
    let handler_result = HandlerResult::from_result(success_result);
    match handler_result {
        HandlerResult::Success(value) => {
            assert_eq!(value, "success value", "HandlerResult should contain success value");
        }
        HandlerResult::Error(_) => {
            panic!("HandlerResult should be success");
        }
    }
    
    // Test HandlerResult::from_result error case
    let error_result: Result<String, Box<dyn std::error::Error + Send + Sync>> = Err("test error".to_string().into());
    let handler_error_result: HandlerResult<String> = HandlerResult::from_result(error_result);
    match handler_error_result {
        HandlerResult::Success(_) => {
            panic!("HandlerResult should be error");
        }
        HandlerResult::Error(error) => {
            assert_eq!(error.code, JSONRPCErrorCode::InternalError as i32, "Error should be internal error");
            assert!(error.data.as_ref().and_then(|d| d.details.as_ref()).map(|s| s.as_str()).is_some(), "Error should have details");
        }
    }
}