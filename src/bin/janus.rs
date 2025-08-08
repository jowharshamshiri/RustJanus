use clap::{Arg, Command};
use std::fs;
use rust_janus::{JanusClient, JanusServer, JanusClientConfig, ServerConfig, ManifestParser, Manifest};
use std::collections::HashMap;
use serde_json::Value;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("janus")
        .version("1.0.0")
        .author("RustJanus Team")
        .about("Rust SOCK_DGRAM Unix Socket Process")
        .arg(
            Arg::new("socket")
                .short('s')
                .long("socket")
                .value_name("PATH")
                .help("Unix socket path")
                .default_value("/tmp/rust-janus.sock"),
        )
        .arg(
            Arg::new("listen")
                .short('l')
                .long("listen")
                .help("Listen for datagrams on socket")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("send-to")
                .long("send-to")
                .value_name("PATH")
                .help("Send datagram to socket path"),
        )
        .arg(
            Arg::new("request")
                .short('c')
                .long("request")
                .value_name("CMD")
                .help("Request to send")
                .default_value("ping"),
        )
        .arg(
            Arg::new("message")
                .short('m')
                .long("message")
                .value_name("MSG")
                .help("Message to send")
                .default_value("hello"),
        )
        .arg(
            Arg::new("manifest")
                .long("manifest")
                .value_name("FILE")
                .help("Manifest file (required for validation)"),
        )
        .get_matches();

    let socket_path = matches.get_one::<String>("socket").unwrap();
    let listen = matches.get_flag("listen");
    let send_to = matches.get_one::<String>("send-to");
    let request = matches.get_one::<String>("request").unwrap();
    let message = matches.get_one::<String>("message").unwrap();
    let manifest_path = matches.get_one::<String>("manifest");
    // Channel removed from protocol

    // Load Manifest if provided
    let manifest = if let Some(manifest_path) = manifest_path {
        let manifest_data = fs::read_to_string(manifest_path)?;
        Some(ManifestParser::from_json(&manifest_data)?)
    } else {
        None
    };

    if let Some(manifest) = &manifest {
        println!("Loaded Manifest version: {}", manifest.version);
    }

    if listen {
        // Server mode - use JanusServer API
        listen_for_datagrams(socket_path, manifest).await?;
    } else if let Some(target_socket) = send_to {
        // Client mode - use JanusClient API
        send_datagram(target_socket, request, message).await?;
    } else {
        eprintln!("Usage: either --listen or --send-to required");
        std::process::exit(1);
    }

    Ok(())
}

async fn listen_for_datagrams(
    socket_path: &str,
    _manifest: Option<Manifest>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listening for SOCK_DGRAM on: {}", socket_path);

    // Create server configuration
    let config = ServerConfig {
        socket_path: socket_path.to_string(),
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
        ..Default::default()
    };

    // Create and start server using library API
    let mut server = JanusServer::new(config);

    // Register built-in request handlers (handled by library)
    // Built-in handlers (ping, echo, get_info, validate, slow_process, manifest) 
    // are automatically registered by the library

    println!("Ready to receive datagrams");

    // Start listening - this handles all the socket logic
    server.start_listening().await?;

    println!("DEBUG: Server started, waiting for completion...");
    
    // Use the new API to wait for completion
    tokio::select! {
        result = server.wait_for_completion() => {
            println!("DEBUG: Server completed: {:?}", result);
            result.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
        _ = tokio::signal::ctrl_c() => {
            println!("DEBUG: Received shutdown signal, stopping server...");
            server.stop();
            println!("DEBUG: Server stopped gracefully");
            Ok(())
        }
    }
}

async fn send_datagram(
    target_socket: &str,
    request: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Sending SOCK_DGRAM to: {}", target_socket);

    // Create client configuration
    let config = JanusClientConfig::default();

    // Create client using library API (channels removed)
    let mut client = JanusClient::new(
        target_socket.to_string(),
        config,
    ).await?;

    // Prepare arguments based on request type
    let mut args = HashMap::new();
    if ["echo", "get_info", "validate", "slow_process"].contains(&request) {
        args.insert("message".to_string(), Value::String(message.to_string()));
    }

    // Send request using library API
    let response = client
        .send_request(
            request,
            if args.is_empty() { None } else { Some(args) },
            Some(std::time::Duration::from_secs(5)),
        )
        .await?;

    println!(
        "Response: Success={}, Result={}",
        response.success,
        serde_json::to_string(&response.result)?
    );

    Ok(())
}