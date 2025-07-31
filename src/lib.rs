//! # RustJanus v2.0 - SwiftJanus Parity
//!
//! Enterprise-grade Unix domain socket API communication library for Rust.
//! This version achieves exact functional parity with SwiftJanus, providing
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
//! use rust_janus::{JanusClient, ApiSpecification, JanusClientConfig};
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
//!     let config = JanusClientConfig::default();
//!     
//!     // Initialize client
//!     let client = JanusClient::new(
//!         "/tmp/my_socket.sock".to_string(),
//!         "my-channel".to_string(),
//!         Some(api_spec),
//!         config
//!     )?;
//!     
//!     // Send command
//!     let mut args = HashMap::new();
//!     args.insert("action".to_string(), json!("process"));
//!     args.insert("data".to_string(), json!("Hello, Server!"));
//!     
//!     let response = client.send_command(
//!         "my-command",
//!         Some(args),
//!         Some(Duration::from_secs(30)),
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
pub mod server;

// Core exports (low-level SOCK_DGRAM socket communication)
pub use core::{CoreJanusClient, SecurityValidator};

// Protocol exports (SOCK_DGRAM API communication layer)
pub use protocol::{
    SocketCommand, SocketResponse, SocketMessage, MessageType,
    JanusClient, TimeoutManager
};

// High-level API exports (simple one-line usage)
pub use server::{JanusServer, JanusCommandHandler};

// Specification exports (API definition layer)
pub use specification::{
    ApiSpecification, ChannelSpec, CommandSpec, ArgumentSpec,
    ValidationSpec, ResponseSpec, ErrorCodeSpec, ModelSpec,
    ApiSpecificationParser, ValidationEngine, ArgumentValidator
};

// Configuration exports
pub use config::JanusClientConfig;

// Error exports
pub use error::{JanusError, SocketError};

// Result type alias
pub type Result<T> = std::result::Result<T, JanusError>;

// Utility exports
pub use utils::{PathUtils, UuidUtils, JsonUtils};

// Re-export common dependencies for convenience
pub use serde::{Deserialize, Serialize};
pub use serde_json::{Value as JsonValue, json};
pub use chrono::{DateTime, Utc};

/// Prelude module for convenient importing
pub mod prelude {
    pub use crate::{
        JanusClient, CoreJanusClient, ApiSpecification, JanusClientConfig,
        // Connection-based classes removed
        SocketCommand, SocketResponse, SocketMessage, MessageType,
        SecurityValidator, TimeoutManager,
        JanusError, SocketError, Result,
        ChannelSpec, CommandSpec, ArgumentSpec, ResponseSpec,
        Deserialize, Serialize, JsonValue, json,
        DateTime, Utc,
    };
}

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Migration indicator - this is v2.0 with SwiftJanus parity
pub const MIGRATION_VERSION: &str = "2.0.0-swift-parity";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert_eq!(NAME, "RustJanus");
        assert_eq!(MIGRATION_VERSION, "2.0.0-swift-parity");
    }
}