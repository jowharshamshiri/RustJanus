use rs_unix_sock_comms::prelude::*;
use rs_unix_sock_comms::protocol::unix_sock_api_client::CommandHandler;
use rs_unix_sock_comms::ApiSpecificationParser;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal;

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

    println!("Starting Rust Unix Socket API Server on: {}", socket_path);

    // Remove existing socket file
    let _ = std::fs::remove_file(&socket_path);

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

    // Create client for handling commands
    let client = UnixSockApiClient::new(
        socket_path.clone(),
        "test".to_string(),
        spec,
        config,
    ).await?;

    // Register ping handler
    let ping_handler: CommandHandler = Arc::new(|_cmd, _args| {
        Box::pin(async move {
            let mut response = HashMap::new();
            response.insert("pong".to_string(), serde_json::Value::Bool(true));
            response.insert("timestamp".to_string(), 
                serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
            Ok(Some(response))
        })
    });
    client.register_command_handler("ping", ping_handler).await?;

    // Register echo handler
    let echo_handler: CommandHandler = Arc::new(|_cmd, args| {
        Box::pin(async move {
            let message = args
                .as_ref()
                .and_then(|a| a.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("No message provided");
                
            let mut response = HashMap::new();
            response.insert("echo".to_string(), serde_json::Value::String(message.to_string()));
            Ok(Some(response))
        })
    });
    client.register_command_handler("echo", echo_handler).await?;

    // Start listening
    client.start_listening().await?;
    
    println!("Rust server listening. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    signal::ctrl_c().await?;
    println!("Shutting down Rust server...");

    Ok(())
}