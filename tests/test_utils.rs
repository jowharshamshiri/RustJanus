use rust_janus::*;
use std::sync::atomic::AtomicUsize;
use std::collections::HashMap;
use tempfile::TempDir;
use std::path::PathBuf;

/// Create a test socket path in a temporary directory
pub fn create_test_socket_path() -> (TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().unwrap();
    let socket_path = temp_dir.path().join("test.sock");
    (temp_dir, socket_path)
}

/// Fetch API specification from a running test server
/// This replaces hardcoded specifications with dynamic fetching for better test accuracy
pub async fn fetch_test_api_spec(server_socket_path: &str) -> ApiSpecification {
    use rust_janus::core::CoreJanusClient;
    use rust_janus::config::JanusClientConfig;
    
    // Create minimal client configuration for spec fetching
    let config = JanusClientConfig {
        max_concurrent_connections: 1,
        max_message_size: 1_000_000,
        connection_timeout: std::time::Duration::from_secs(5),
        max_pending_commands: 10,
        max_command_handlers: 10,
        enable_resource_monitoring: false,
        max_channel_name_length: 128,
        max_command_name_length: 128,
        max_args_data_size: 500_000,
    };
    
    // Create core client for spec fetching
    let core_client = CoreJanusClient::new(server_socket_path.to_string(), config)
        .expect("Failed to create core client for spec fetching");
    
    // Generate response socket path
    let response_socket_path = core_client.generate_response_socket_path();
    
    // Create spec command
    let spec_command = serde_json::json!({
        "command": "spec",
        "reply_to": response_socket_path
    });
    
    let command_data = serde_json::to_vec(&spec_command)
        .expect("Failed to serialize spec command");
    
    // Send spec command to server
    let response_data = core_client
        .send_datagram(&command_data, &response_socket_path)
        .await
        .expect("Failed to fetch specification from server");
    
    // Parse response JSON
    let response: serde_json::Value = serde_json::from_slice(&response_data)
        .expect("Failed to parse server response");
    
    // Check for error in response
    if let Some(error) = response.get("error") {
        panic!("Server returned error when fetching spec: {}", error);
    }
    
    // Extract specification from response
    let spec_data = response.get("result")
        .expect("Missing 'result' field in spec response");
    
    // Parse API specification
    ApiSpecificationParser::from_json(&serde_json::to_string(spec_data).unwrap())
        .expect("Failed to parse API specification from server response")
}

/// Load test API specification from test-spec.json file
pub fn load_test_api_spec() -> ApiSpecification {
    let spec_path = "../../test-spec.json";
    let spec_data = std::fs::read_to_string(spec_path)
        .expect("Failed to read test-spec.json");
    
    let spec_json: serde_json::Value = serde_json::from_str(&spec_data)
        .expect("Failed to parse test-spec.json");
    
    ApiSpecificationParser::from_json(&spec_data)
        .expect("Failed to parse API specification from test-spec.json")
}


/// Create test client configuration with secure defaults
pub fn create_test_config() -> JanusClientConfig {
    JanusClientConfig {
        max_concurrent_connections: 10,
        max_message_size: 1_000_000,  // 1MB
        connection_timeout: std::time::Duration::from_secs(5),
        max_pending_commands: 100,
        max_command_handlers: 50,
        enable_resource_monitoring: true,
        max_channel_name_length: 128,
        max_command_name_length: 128,
        max_args_data_size: 500_000,  // 500KB
    }
}

/// Create a valid socket path for testing
pub fn create_valid_socket_path() -> String {
    "/tmp/test_socket.sock".to_string()
}

/// Create malicious socket paths for security testing
pub fn get_malicious_socket_paths() -> Vec<String> {
    vec![
        "/tmp/../etc/passwd".to_string(),
        "/tmp/../../usr/bin/sh".to_string(),
        "/tmp/../../../root/.ssh/id_rsa".to_string(),
        "/var/run/../../etc/shadow".to_string(),
        "/var/tmp/../../../proc/version".to_string(),
        "/tmp/../../../../dev/null".to_string(),
        "/tmp/../../../../../../../../../../etc/passwd".to_string(),
        "/../../../etc/passwd".to_string(),
        "/tmp/./../../etc/passwd".to_string(),
        "/tmp/./../../../etc/passwd".to_string(),
    ]
}

/// Create malicious channel IDs for injection testing
pub fn get_malicious_channel_ids() -> Vec<String> {
    vec![
        "'; DROP TABLE users; --".to_string(),  // SQL injection
        "$(rm -rf /)".to_string(),              // Command injection
        "`cat /etc/passwd`".to_string(),        // Command injection
        "channel && rm -rf /".to_string(),      // Command chaining
        "channel | cat /etc/shadow".to_string(), // Pipe injection
        "<script>alert('xss')</script>".to_string(), // XSS
        "../../../etc/passwd".to_string(),       // Path traversal
        "channel\0injection".to_string(),        // Null byte
        "channel\r\ninjection".to_string(),      // CRLF injection
        "очень_длинное_имя_канала_с_unicode_символами_🚀".to_string(), // Unicode
    ]
}

/// Create malformed JSON patterns for protocol testing
pub fn get_malformed_json_patterns() -> Vec<&'static str> {
    vec![
        r#"{"invalid": json syntax"#,           // Unclosed brace
        r#"{"key": "value",}"#,                 // Trailing comma
        r#"{"key": undefined}"#,                // Undefined value
        r#"{"key": 'single_quotes'}"#,          // Single quotes
        r#"{key: "no_quotes_on_key"}"#,         // Unquoted key
        r#"{"duplicate": "key", "duplicate": "value"}"#, // Duplicate keys
        r#"{"infinite": Infinity}"#,            // JavaScript Infinity
    ]
}

/// Create UTF-8 test cases
pub fn get_utf8_test_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("ASCII", "Hello World"),
        ("Unicode", "Hello 世界 🌍"),
        ("Emoji", "🚀🎉🔥💯"),
        ("Chinese", "你好世界"),
        ("Arabic", "مرحبا بالعالم"),
        ("Mixed", "Hello 🌍 世界 مرحبا"),
        ("Zalgo", "Z̸̧̦̥͔̲̪̮̘̺̰̈́̀̾̇͌̚a̸̢̜̟̜̯̗̘̞̅̾̊̐l̴̡̳̜̮̰̜̈́̓̋̾̕g̶̭̰̰̱̃̓ǫ̸̱̠̫̗̇̈́̌"),
    ]
}

/// Create invalid UTF-8 byte sequences
pub fn get_invalid_utf8_sequences() -> Vec<Vec<u8>> {
    vec![
        vec![0xFF, 0xFE],           // Invalid start bytes
        vec![0x80, 0x80],           // Continuation without start
        vec![0xC0, 0x80],           // Overlong encoding
        vec![0xED, 0xA0, 0x80],     // Surrogate half
        vec![0xF4, 0x90, 0x80, 0x80], // Above Unicode range
    ]
}

/// Create command arguments for testing
pub fn create_test_args() -> HashMap<String, serde_json::Value> {
    let mut args = HashMap::new();
    args.insert("message".to_string(), serde_json::Value::String("test_value".to_string()));
    args
}

/// Create large test data for stress testing
pub fn create_large_test_data(size_kb: usize) -> String {
    "x".repeat(size_kb * 1024)
}

/// Create nested test data for validation
pub fn create_nested_test_data() -> serde_json::Value {
    serde_json::json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "deep_value": "nested_data",
                        "array": [1, 2, 3, {"nested": "object"}],
                        "unicode": "🚀",
                        "number": 42.5
                    }
                }
            }
        }
    })
}

/// Measure execution time
pub async fn measure_time<F, T>(operation: F) -> (T, std::time::Duration)
where
    F: std::future::Future<Output = T>,
{
    let start = std::time::Instant::now();
    let result = operation.await;
    let duration = start.elapsed();
    (result, duration)
}

/// Create timeout handler that counts invocations
/// Note: SOCK_DGRAM architecture uses different timeout patterns
pub fn create_counting_timeout_handler() -> std::sync::Arc<AtomicUsize> {
    // Return just the counter for SOCK_DGRAM testing
    std::sync::Arc::new(AtomicUsize::new(0))
}

/// Create large test data map for datagram size testing
pub fn create_large_test_data_map(size_multiplier: usize) -> HashMap<String, serde_json::Value> {
    let mut args = HashMap::new();
    
    // Create a large string
    let large_string = "x".repeat(size_multiplier);
    args.insert("large_data".to_string(), serde_json::Value::String(large_string));
    
    // Add multiple entries to increase size
    for i in 0..size_multiplier.min(100) {
        args.insert(format!("data_{}", i), serde_json::Value::String(format!("value_{}", i)));
    }
    
    args
}