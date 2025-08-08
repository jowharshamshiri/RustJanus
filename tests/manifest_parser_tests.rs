use rust_janus::*;
mod test_utils;
use test_utils::*;

/// Manifest Parser Tests (12 tests) - SwiftJanus parity
/// Tests JSON/YAML parsing, validation, error handling

#[tokio::test]
async fn test_parse_json_manifest() {
    let manifest = load_test_manifest();
    let json_str = ManifestParser::to_json(&manifest).unwrap();
    
    let parsed = ManifestParser::from_json(&json_str).unwrap();
    assert_eq!(parsed.version, manifest.version);
    assert_eq!(parsed.channels.len(), manifest.channels.len());
}

#[cfg(feature = "yaml-support")]
#[tokio::test]
async fn test_parse_yaml_manifest() {
    let manifest = load_test_manifest();
    let yaml_str = ManifestParser::to_yaml(&manifest).unwrap();
    
    let parsed = ManifestParser::from_yaml(&yaml_str).unwrap();
    assert_eq!(parsed.version, manifest.version);
    assert_eq!(parsed.channels.len(), manifest.channels.len());
}

#[tokio::test]
async fn test_validate_valid_manifest() {
    let manifest = load_test_manifest();
    let result = ManifestParser::validate(&manifest);
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_parse_invalid_json() {
    let invalid_json = r#"{"invalid": json, "missing": quote}"#;
    let result = ManifestParser::from_json(invalid_json);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_empty_manifest() {
    let empty_manifest = r#"{"version": "1.0.0", "channels": {}}"#;
    let parsed = ManifestParser::from_json(empty_manifest).unwrap();
    assert_eq!(parsed.version, "1.0.0");
    assert!(parsed.channels.is_empty());
}

#[tokio::test]
async fn test_validate_missing_version() {
    let invalid_manifest = r#"{"channels": {}}"#;
    let result = ManifestParser::from_json(invalid_manifest);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_invalid_channel_definition() {
    let invalid_manifest = r#"{
        "version": "1.0.0",
        "channels": {
            "test": {
                "requests": {
                    "invalid_request": {}
                }
            }
        }
    }"#;
    let result = ManifestParser::from_json(invalid_manifest);
    // Should either fail parsing or validation
    if let Ok(manifest) = result {
        let validation_result = ManifestParser::validate(&manifest);
        assert!(validation_result.is_err());
    }
}

#[tokio::test]
async fn test_reserved_request_rejection() {
    let manifest_with_reserved = "{
        \"version\": \"1.0.0\",
        \"channels\": {
            \"test\": {
                \"requests\": {
                    \"ping\": {
                        \"description\": \"Should be rejected\"
                    }
                }
            }
        }
    }";
    let result = ManifestParser::from_json(manifest_with_reserved);
    if let Ok(manifest) = result {
        let validation_result = ManifestParser::validate(&manifest);
        assert!(validation_result.is_err(), "Should reject reserved request 'ping'");
    }
}

#[tokio::test]
async fn test_serialize_and_deserialize_roundtrip() {
    let original_manifest = load_test_manifest();
    let json_str = ManifestParser::to_json(&original_manifest).unwrap();
    let deserialized = ManifestParser::from_json(&json_str).unwrap();
    
    // Test that serialization->deserialization preserves structure
    let second_json = ManifestParser::to_json(&deserialized).unwrap();
    let second_deserialized = ManifestParser::from_json(&second_json).unwrap();
    
    assert_eq!(deserialized.version, second_deserialized.version);
    assert_eq!(deserialized.channels.len(), second_deserialized.channels.len());
}

#[tokio::test]
async fn test_request_argument_validation() {
    let manifest_with_args = "{
        \"version\": \"1.0.0\",
        \"channels\": {
            \"test\": {
                \"description\": \"Test channel\",
                \"requests\": {
                    \"test_request\": {
                        \"description\": \"Test request with arguments\",
                        \"arguments\": {
                            \"required_param\": {
                                \"type\": \"string\",
                                \"required\": true
                            },
                            \"optional_param\": {
                                \"type\": \"integer\",
                                \"required\": false
                            }
                        },
                        \"response\": {
                            \"type\": \"object\",
                            \"properties\": {
                                \"result\": {\"type\": \"string\"}
                            }
                        }
                    }
                }
            }
        }
    }";
    
    let parsed = ManifestParser::from_json(manifest_with_args).unwrap();
    let validation_result = ManifestParser::validate(&parsed);
    assert!(validation_result.is_ok());
}

#[tokio::test]
async fn test_model_reference_validation() {
    let manifest_with_models = "{
        \"version\": \"1.0.0\",
        \"models\": {
            \"User\": {
                \"type\": \"object\",
                \"properties\": {
                    \"name\": {\"type\": \"string\", \"required\": true},
                    \"age\": {\"type\": \"integer\", \"required\": false}
                }
            }
        },
        \"channels\": {
            \"test\": {
                \"description\": \"Test channel\",
                \"requests\": {
                    \"get_user\": {
                        \"description\": \"Get user by ID\",
                        \"arguments\": {
                            \"id\": {
                                \"type\": \"string\",
                                \"required\": true
                            }
                        },
                        \"response\": {
                            \"type\": \"object\",
                            \"$ref\": \"#/models/User\"
                        }
                    }
                }
            }
        }
    }";
    
    let parsed = ManifestParser::from_json(manifest_with_models).unwrap();
    let validation_result = ManifestParser::validate(&parsed);
    assert!(validation_result.is_ok());
}