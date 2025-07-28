//! # RustUnixSockAPI v2.0 - SwiftUnixSockAPI Parity
//!
//! Enterprise-grade Unix domain socket API communication library for Rust.
//! This version achieves exact functional parity with SwiftUnixSockAPI, providing
//! stateless communication, comprehensive security, and API specification-driven development.
//!
//! ## Features
//!
//! - **Stateless Communication**: Ephemeral connections with UUID tracking
//! - **Security Framework**: Path validation, resource limits, attack prevention
//! - **API Specification**: JSON/YAML specification-driven development
//! - **Bilateral Timeouts**: Caller and handler timeout management
//! - **Connection Pooling**: Efficient connection reuse with limits
//! - **Comprehensive Validation**: Input sanitization and type checking
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rs_unix_sock_comms::{UnixSockApiClient, ApiSpecification, UnixSockApiClientConfig};
//! use std::collections::HashMap;
//! use serde_json::json;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load API specification
//!     let api_spec = ApiSpecification::from_file("api-spec.json").await?;
//!     
//!     // Create client configuration
//!     let config = UnixSockApiClientConfig::default();
//!     
//!     // Initialize client
//!     let client = UnixSockApiClient::new(
//!         "/tmp/my_socket.sock".to_string(),
//!         "my-channel".to_string(),
//!         api_spec,
//!         config
//!     ).await?;
//!     
//!     // Send command
//!     let mut args = HashMap::new();
//!     args.insert("action".to_string(), json!("process"));
//!     args.insert("data".to_string(), json!("Hello, Server!"));
//!     
//!     let response = client.send_command(
//!         "my-command",
//!         Some(args),
//!         Duration::from_secs(30),
//!         None
//!     ).await?;
//!     
//!     if response.success {
//!         println!("Success: {:?}", response.result);
//!     } else {
//!         println!("Error: {:?}", response.error);
//!     }
//!     
//!     Ok(())
//! }
//! ```

pub mod core;
pub mod protocol;
pub mod specification;
pub mod config;
pub mod error;
pub mod utils;

// Core exports (low-level socket communication)
pub use core::{UnixSocketClient, ConnectionPool, MessageFrame, SecurityValidator};

// Protocol exports (API communication layer)
pub use protocol::{
    SocketCommand, SocketResponse, SocketMessage, MessageType,
    UnixSockApiClient, TimeoutManager, TimeoutHandler,
    CommandHandler, CommandHandlerRegistry
};

// Specification exports (API definition layer)
pub use specification::{
    ApiSpecification, ChannelSpec, CommandSpec, ArgumentSpec,
    ValidationSpec, ResponseSpec, ErrorCodeSpec, ModelSpec,
    ApiSpecificationParser, ValidationEngine, ArgumentValidator
};

// Configuration exports
pub use config::UnixSockApiClientConfig;

// Error exports
pub use error::{UnixSockApiError, SocketError, Result};

// Utility exports
pub use utils::{PathUtils, UuidUtils, JsonUtils};

// Re-export common dependencies for convenience
pub use serde::{Deserialize, Serialize};
pub use serde_json::{Value as JsonValue, json};
pub use chrono::{DateTime, Utc};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Migration indicator - this is v2.0 with SwiftUnixSockAPI parity
pub const MIGRATION_VERSION: &str = "2.0.0-swift-parity";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "RustUnixSockAPI");
        assert_eq!(MIGRATION_VERSION, "2.0.0-swift-parity");
    }
}