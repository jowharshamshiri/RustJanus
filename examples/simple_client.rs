use rust_unix_sock_api::prelude::*;
use rust_unix_sock_api::ApiSpecificationParser;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
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
    
    let spec_data = std::fs::read_to_string(&spec_path)?;
    let spec = ApiSpecificationParser::from_json(&spec_data)?;

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