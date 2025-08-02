use rust_janus::*;
mod test_utils;
use test_utils::*;

/// Edge Cases Tests (10 tests) - SwiftJanus parity
/// Tests null values, nested data, large values, special characters

#[tokio::test]
async fn test_anyccodable_with_null_values() {
    let command = SocketCommand::new(
        "test".to_string(),
        "echo".to_string(),
        Some({
            let mut args = std::collections::HashMap::new();
            args.insert("null_value".to_string(), serde_json::Value::Null);
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&command).unwrap();
    let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
    
    let binding = parsed.args.unwrap();
    let null_value = binding.get("null_value").unwrap();
    assert!(null_value.is_null());
}

#[tokio::test]
async fn test_deeply_nested_json_structures() {
    let nested_value = serde_json::json!({
        "level1": {
            "level2": {
                "level3": {
                    "level4": {
                        "data": "deeply nested value"
                    }
                }
            }
        }
    });
    
    let command = SocketCommand::new(
        "test".to_string(),
        "process_nested".to_string(),
        Some({
            let mut args = std::collections::HashMap::new();
            args.insert("nested".to_string(), nested_value);
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&command).unwrap();
    let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
    
    assert!(parsed.args.is_some());
    let args = parsed.args.unwrap();
    let nested = args.get("nested").unwrap();
    assert_eq!(nested["level1"]["level2"]["level3"]["level4"]["data"], "deeply nested value");
}

#[tokio::test]
async fn test_large_string_values() {
    let large_string = "x".repeat(10000); // 10KB string
    
    let command = SocketCommand::new(
        "test".to_string(),
        "process_large".to_string(),
        Some({
            let mut args = std::collections::HashMap::new();
            args.insert("large_data".to_string(), serde_json::Value::String(large_string.clone()));
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&command).unwrap();
    let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
    
    let args = parsed.args.unwrap();
    let large_value = args.get("large_data").unwrap().as_str().unwrap();
    assert_eq!(large_value.len(), 10000);
    assert_eq!(large_value, large_string);
}

#[tokio::test]
async fn test_special_unicode_characters() {
    let unicode_test_cases = vec![
        "🚀", // Emoji
        "你好", // Chinese
        "مرحبا", // Arabic
        "🇺🇸🇯🇵", // Flag emojis
        "©®™", // Symbols
        "\u{200B}", // Zero-width space
        "\n\t\r", // Control characters
    ];
    
    for (i, test_case) in unicode_test_cases.iter().enumerate() {
        let command = SocketCommand::new(
            "test".to_string(),
            "unicode_test".to_string(),
            Some({
                let mut args = std::collections::HashMap::new();
                args.insert(format!("unicode_{}", i), serde_json::Value::String(test_case.to_string()));
                args
            }),
            None,
        );
        
        let json_str = serde_json::to_string(&command).unwrap();
        let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
        
        let args = parsed.args.unwrap();
        let unicode_value = args.get(&format!("unicode_{}", i)).unwrap().as_str().unwrap();
        assert_eq!(unicode_value, *test_case);
    }
}

#[tokio::test]
async fn test_array_with_mixed_types() {
    let mixed_array = serde_json::json!([
        42,
        "string",
        true,
        null,
        {"nested": "object"},
        [1, 2, 3]
    ]);
    
    let command = SocketCommand::new(
        "test".to_string(),
        "mixed_array".to_string(),
        Some({
            let mut args = std::collections::HashMap::new();
            args.insert("mixed".to_string(), mixed_array);
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&command).unwrap();
    let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
    
    let args = parsed.args.unwrap();
    let mixed = args.get("mixed").unwrap().as_array().unwrap();
    assert_eq!(mixed.len(), 6);
    assert_eq!(mixed[0], 42);
    assert_eq!(mixed[1], "string");
    assert_eq!(mixed[2], true);
    assert!(mixed[3].is_null());
}

#[tokio::test]
async fn test_empty_values_handling() {
    let command = SocketCommand::new(
        "test".to_string(),
        "empty_test".to_string(),
        Some({
            let mut args = std::collections::HashMap::new();
            args.insert("empty_string".to_string(), serde_json::Value::String("".to_string()));
            args.insert("empty_array".to_string(), serde_json::Value::Array(vec![]));
            args.insert("empty_object".to_string(), serde_json::Value::Object(serde_json::Map::new()));
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&command).unwrap();
    let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
    
    let args = parsed.args.unwrap();
    assert_eq!(args.get("empty_string").unwrap().as_str().unwrap(), "");
    assert_eq!(args.get("empty_array").unwrap().as_array().unwrap().len(), 0);
    assert_eq!(args.get("empty_object").unwrap().as_object().unwrap().len(), 0);
}

#[tokio::test]
async fn test_numeric_edge_cases() {
    let command = SocketCommand::new(
        "test".to_string(),
        "numeric_test".to_string(),
        Some({
            let mut args = std::collections::HashMap::new();
            args.insert("max_int".to_string(), serde_json::Value::Number(serde_json::Number::from(i64::MAX)));
            args.insert("min_int".to_string(), serde_json::Value::Number(serde_json::Number::from(i64::MIN)));
            args.insert("zero".to_string(), serde_json::Value::Number(serde_json::Number::from(0)));
            args.insert("float".to_string(), serde_json::json!(3.14159));
            args
        }),
        None,
    );
    
    let json_str = serde_json::to_string(&command).unwrap();
    let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
    
    let args = parsed.args.unwrap();
    assert_eq!(args.get("max_int").unwrap().as_i64().unwrap(), i64::MAX);
    assert_eq!(args.get("min_int").unwrap().as_i64().unwrap(), i64::MIN);
    assert_eq!(args.get("zero").unwrap().as_i64().unwrap(), 0);
    assert!((args.get("float").unwrap().as_f64().unwrap() - 3.14159).abs() < 1e-6);
}

#[tokio::test]
async fn test_malformed_json_recovery() {
    let malformed_json_cases = vec![
        r#"{"incomplete": "#, // Incomplete JSON
        r#"{"duplicate": 1, "duplicate": 2}"#, // Duplicate keys
        r#"{"trailing": "comma",}"#, // Trailing comma
        r#"{"unescaped": "quote"inside"}"#, // Unescaped quotes
    ];
    
    for malformed in malformed_json_cases {
        let result: Result<SocketCommand, _> = serde_json::from_str(malformed);
        assert!(result.is_err(), "Should fail parsing malformed JSON: {}", malformed);
    }
}

#[tokio::test]
async fn test_command_id_edge_cases() {
    let edge_case_ids = vec![
        "".to_string(),
        " ".to_string(),
        "command-with-hyphens".to_string(),
        "command_with_underscores".to_string(),
        "CommandWithCamelCase".to_string(),
        "123numeric_start".to_string(),
        "very_long_command_name_that_exceeds_typical_lengths_and_tests_boundary_conditions".to_string(),
    ];
    
    for test_id in edge_case_ids {
        let command = SocketCommand::new(
            "test".to_string(),
            test_id.clone(),
            None,
            None,
        );
        
        let json_str = serde_json::to_string(&command).unwrap();
        let parsed: SocketCommand = serde_json::from_str(&json_str).unwrap();
        
        assert_eq!(parsed.command, test_id);
    }
}