use rust_unix_sock_api::*;
use std::collections::HashMap;

#[test]
fn test_json_parsing_error_logging() {
    // Initialize logging for test
    let _ = env_logger::try_init();
    
    // Test empty JSON
    let result = ApiSpecificationParser::from_json("");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("input string is empty"));
    
    // Test malformed JSON
    let malformed_json = r#"{"version": "1.0.0", "channels": }"#;
    let result = ApiSpecificationParser::from_json(malformed_json);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("JSON parsing error"));
    
    // Test invalid data structure
    let invalid_structure = r#"{"version": 123, "channels": {}}"#;
    let result = ApiSpecificationParser::from_json(invalid_structure);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("JSON parsing error"));
}

#[test]
fn test_validation_error_logging() {
    // Initialize logging for test
    let _ = env_logger::try_init();
    
    // Test missing version
    let json_missing_version = r#"{"channels": {"test": {"description": "Test", "commands": {"ping": {"description": "Ping", "response": {"type": "object"}}}}}}"#;
    let result = ApiSpecificationParser::load_and_validate_json(json_missing_version);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("version is required"));
    
    // Test invalid version format
    let json_invalid_version = r#"{"version": "invalid", "channels": {"test": {"description": "Test", "commands": {"ping": {"description": "Ping", "response": {"type": "object"}}}}}}"#;
    let result = ApiSpecificationParser::load_and_validate_json(json_invalid_version);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Invalid version format"));
    
    // Test missing channels
    let json_no_channels = r#"{"version": "1.0.0", "channels": {}}"#;
    let result = ApiSpecificationParser::load_and_validate_json(json_no_channels);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("at least one channel"));
}

#[test]
fn test_validation_summary() {
    // Initialize logging for test
    let _ = env_logger::try_init();
    
    // Test empty API spec
    let mut api_spec = ApiSpecification::new("".to_string());
    let summary = ApiSpecificationParser::get_validation_summary(&api_spec);
    assert!(summary.contains("Missing version"));
    assert!(summary.contains("No channels defined"));
    
    // Test valid API spec
    api_spec.version = "1.0.0".to_string();
    let mut channel = ChannelSpec::new("Test channel".to_string());
    let command = CommandSpec::new("Test command".to_string(), ResponseSpec::new("object".to_string()));
    channel.add_command("test".to_string(), command);
    api_spec.add_channel("test".to_string(), channel);
    
    let summary = ApiSpecificationParser::get_validation_summary(&api_spec);
    assert!(summary.contains("appears to be valid"));
}

#[tokio::test]
async fn test_file_loading_error_logging() {
    // Initialize logging for test
    let _ = env_logger::try_init();
    
    // Test non-existent file
    let result = ApiSpecificationParser::from_file("non_existent_file.json").await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Failed to access file"));
    
    // Test empty file path
    let result = ApiSpecificationParser::from_file("").await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("File path cannot be empty"));
}

#[test]
fn test_successful_parsing_logging() {
    // Initialize logging for test
    let _ = env_logger::try_init();
    
    // Test valid JSON parsing with logging
    let valid_json = r#"{
        "version": "1.0.0",
        "channels": {
            "test": {
                "description": "Test channel",
                "commands": {
                    "ping": {
                        "description": "Ping command",
                        "response": {
                            "type": "object"
                        }
                    }
                }
            }
        }
    }"#;
    
    let result = ApiSpecificationParser::load_and_validate_json(valid_json);
    assert!(result.is_ok());
    let api_spec = result.unwrap();
    assert_eq!(api_spec.version, "1.0.0");
    assert_eq!(api_spec.channels.len(), 1);
}

#[cfg(feature = "yaml-support")]
#[test]
fn test_yaml_parsing_error_logging() {
    // Initialize logging for test
    let _ = env_logger::try_init();
    
    // Test empty YAML
    let result = ApiSpecificationParser::from_yaml("");
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("input string is empty"));
    
    // Test malformed YAML
    let malformed_yaml = r#"
version: 1.0.0
channels:
  test:
    description: Test
    commands
      ping:
        description: Ping
"#;
    let result = ApiSpecificationParser::from_yaml(malformed_yaml);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("YAML parsing error"));
}