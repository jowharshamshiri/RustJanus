use rust_janus::server::{JanusServer, ServerConfig};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    println!("Starting RustJanus server test...");

    let config = ServerConfig {
        socket_path: "/tmp/rust_janus_test.sock".to_string(),
        max_connections: 100,
        default_timeout: 30,
        max_message_size: 65536,
        cleanup_on_start: true,
        cleanup_on_shutdown: true,
    };

    let mut server = JanusServer::new(config);

    // Start the server
    match server.start_listening().await {
        Ok(()) => {
            println!("Server started successfully");
        }
        Err(e) => {
            eprintln!("Failed to start server: {:?}", e);
            return;
        }
    }

    println!("Server is now listening on /tmp/rust_janus_test.sock");
    println!("Waiting for requests...");

    // Keep the server running for 30 seconds
    for i in 1..=30 {
        if i % 5 == 0 {
            println!("Server still running... ({}/30 seconds)", i);
        }
        sleep(Duration::from_secs(1)).await;
    }

    println!("Stopping server...");
    server.stop();
    println!("Server stopped");
}