use rs_unix_sock_comms::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let socket_path = if args.len() > 2 && args[1] == "--socket-path" {
        args[2].clone()
    } else {
        "/tmp/rust_test_server.sock".to_string()
    };

    println!("Connecting Rust client to: {}", socket_path);

    // Create API specification (matching server)
    let mut spec = ApiSpecification::new("1.0.0".to_string());
    let mut channel = ChannelSpec::new("Test channel".to_string());
    
    // Add ping command
    let ping_response = ResponseSpec::new("object".to_string())
        .with_properties({
            let mut props = HashMap::new();
            props.insert("pong".to_string(), ArgumentSpec::new("boolean".to_string())
                .with_description("Ping response".to_string()));
            props.insert("timestamp".to_string(), ArgumentSpec::new("string".to_string())
                .with_description("Response timestamp".to_string()));
            props
        });
    let ping_cmd = CommandSpec::new("Simple ping command".to_string(), ping_response);
    channel.commands.insert("ping".to_string(), ping_cmd);
    
    // Add echo command
    let echo_response = ResponseSpec::new("object".to_string())
        .with_properties({
            let mut props = HashMap::new();
            props.insert("echo".to_string(), ArgumentSpec::new("string".to_string())
                .with_description("Echoed message".to_string()));
            props
        });
    let echo_cmd = CommandSpec::new("Echo back input".to_string(), echo_response);
    channel.commands.insert("echo".to_string(), echo_cmd);
    
    spec.add_channel("test".to_string(), channel);

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