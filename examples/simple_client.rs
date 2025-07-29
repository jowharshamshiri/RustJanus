use rust_unix_sock_api::prelude::*;
use rust_unix_sock_api::ApiSpecificationParser;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging for better error diagnostics
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
    
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let socket_path = if args.len() > 1 && args[1].starts_with("--socket-path=") {
        args[1].trim_start_matches("--socket-path=").to_string()
    } else if args.len() > 2 && args[1] == "--socket-path" {
        args[2].clone()
    } else {
        "/tmp/rust_test_server.sock".to_string()
    };

    println!("Connecting Rust client to: {}", socket_path);

    // Load API specification from file
    let spec_path = if args.len() > 1 && args[1].starts_with("--spec=") {
        args[1].trim_start_matches("--spec=").to_string()
    } else if args.len() > 3 && args[2] == "--spec" {
        args[3].clone()
    } else {
        "test-api-spec.json".to_string()
    };
    
    // Load and validate API specification with enhanced error logging
    let spec = match std::fs::read_to_string(&spec_path) {
        Ok(spec_data) => {
            println!("Loaded API specification file: {} ({} bytes)", spec_path, spec_data.len());
            
            // Parse and validate the specification
            match ApiSpecificationParser::load_and_validate_json(&spec_data) {
                Ok(spec) => {
                    println!("✓ API specification parsed and validated successfully");
                    println!("  Version: {}", spec.version);
                    println!("  Channels: {}", spec.channels.len());
                    spec
                },
                Err(e) => {
                    eprintln!("✗ Failed to parse API specification: {}", e);
                    
                    // Try to provide a validation summary for diagnostic purposes
                    if let Ok(partial_spec) = ApiSpecificationParser::from_json(&spec_data) {
                        let summary = ApiSpecificationParser::get_validation_summary(&partial_spec);
                        eprintln!("Validation summary:\n{}", summary);
                    }
                    
                    return Err(e.into());
                }
            }
        },
        Err(e) => {
            eprintln!("✗ Failed to read API specification file '{}': {}", spec_path, e);
            return Err(e.into());
        }
    };

    // Create configuration
    let config = UnixSockApiClientConfig::default();

    // Create client
    let client = UnixSockApiClient::new(
        socket_path,
        "test".to_string(),
        spec,
        config,
    ).await?;

    println!("Testing ping command...");
    
    // Test ping command
    match client.send_command("ping", None, Duration::from_secs(5), None).await {
        Ok(response) => {
            println!("Ping response: {:?}", response);
            if response.success {
                println!("✓ Ping test passed");
            } else {
                println!("✗ Ping test failed: {:?}", response.error);
                return Ok(());
            }
        }
        Err(e) => {
            println!("✗ Ping test error: {:?}", e);
            return Ok(());
        }
    }

    println!("Testing echo command...");
    
    // Test echo command
    let mut echo_args = HashMap::new();
    echo_args.insert("message".to_string(), 
        serde_json::Value::String("Hello from Rust client!".to_string()));
    
    match client.send_command("echo", Some(echo_args), Duration::from_secs(5), None).await {
        Ok(response) => {
            println!("Echo response: {:?}", response);
            if response.success {
                println!("✓ Echo test passed");
            } else {
                println!("✗ Echo test failed: {:?}", response.error);
                return Ok(());
            }
        }
        Err(e) => {
            println!("✗ Echo test error: {:?}", e);
            return Ok(());
        }
    }

    println!("All Rust client tests completed successfully! 🎉");
    
    Ok(())
}