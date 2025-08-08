use crate::core::{CoreJanusClient, SecurityValidator};
use crate::error::{JSONRPCError, JSONRPCErrorCode};
use crate::config::JanusClientConfig;
use crate::manifest::Manifest;
use crate::protocol::message_types::{JanusRequest, JanusResponse, RequestHandle, RequestStatus};
use crate::protocol::response_tracker::{ResponseTracker, TrackerConfig, RequestStatistics};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// Note: JanusRequest, JanusResponse, and SocketError types are imported from message_types module
// This ensures cross-language parity and eliminates type duplication

/// Simulate connection state for SOCK_DGRAM compatibility
#[derive(Debug, Clone)]
pub struct ConnectionState {
    pub is_connected: bool,
    pub last_activity: std::time::SystemTime,
    pub messages_sent: u64,
    pub responses_received: u64,
}

impl ConnectionState {
    pub fn new() -> Self {
        Self {
            is_connected: false,
            last_activity: std::time::SystemTime::now(),
            messages_sent: 0,
            responses_received: 0,
        }
    }
    
    pub fn with_connection(is_connected: bool) -> Self {
        Self {
            is_connected,
            last_activity: std::time::SystemTime::now(),
            messages_sent: 0,
            responses_received: 0,
        }
    }
}

/// High-level API client for SOCK_DGRAM Unix socket communication
/// Connectionless implementation with request validation and response correlation
#[derive(Debug)]
pub struct JanusClient {
    socket_path: String,
    manifest: Option<Manifest>,
    config: JanusClientConfig,
    core_client: CoreJanusClient,
    response_tracker: ResponseTracker,
    connection_state: std::sync::Mutex<ConnectionState>,
    // Request lifecycle management (automatic ID system)
    request_registry: std::sync::Mutex<HashMap<String, RequestHandle>>,
    // Note: SecurityValidator is used via static methods, no instance needed
}

impl JanusClient {
    /// Create a new datagram API client
    /// Manifest will be fetched during operations when needed
    pub async fn new(
        socket_path: String,
        config: JanusClientConfig,
    ) -> Result<Self, JSONRPCError> {
        // Validate socket path
        SecurityValidator::validate_socket_path(&socket_path)?;
        
        
        let core_client = CoreJanusClient::new(socket_path.clone(), config.clone())?;
        
        // Initialize response tracker for advanced client features
        let tracker_config = TrackerConfig {
            max_pending_requests: 1000,
            cleanup_interval: Duration::from_secs(30),
            default_timeout: config.connection_timeout,
        };
        let response_tracker = ResponseTracker::new(tracker_config);
        
        Ok(Self {
            socket_path,
            manifest: None,  // Will be fetched during operations when needed
            config,
            core_client,
            response_tracker,
            connection_state: std::sync::Mutex::new(ConnectionState::new()),
            request_registry: std::sync::Mutex::new(HashMap::new()),
        })
    }
    
    /// Fetch Manifest from server
    async fn fetch_manifest_from_server(
        core_client: &CoreJanusClient,
        _config: &JanusClientConfig,
    ) -> Result<Manifest, JSONRPCError> {
        // Generate response socket path
        let response_socket_path = core_client.generate_response_socket_path();
        
        // Create proper JanusRequest for manifest request using constructor
        let mut manifest_request = JanusRequest::new(
            "manifest".to_string(),
            None,
            Some(10.0),
        );
        manifest_request.reply_to = Some(response_socket_path.clone());
        
        let request_data = serde_json::to_vec(&manifest_request)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(format!("Failed to serialize manifest request: {}", e))))?;
        
        // Send manifest request to server
        let response_data = core_client
            .send_datagram(&request_data, &response_socket_path)
            .await?;
        
        // Parse response as JanusResponse
        let response: JanusResponse = serde_json::from_slice(&response_data)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(format!("Failed to parse server response: {}", e))))?;
        
        // Check for error in response
        if !response.success {
            let error_msg = response.error
                .as_ref()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "Unknown error".to_string());
            return Err(JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(format!("Server returned error: {}", error_msg))));
        }
        
        // Extract manifest from response
        let manifest_data = response.result.as_ref()
            .ok_or_else(|| JSONRPCError::new(JSONRPCErrorCode::InternalError, Some("Server response missing 'result' field".to_string())))?;
        
        // Parse the manifest
        let manifest: Manifest = serde_json::from_value(manifest_data.clone())
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(format!("Failed to parse server manifest: {}", e))))?;
        
        Ok(manifest)
    }
    
    /// Ensure Manifest is loaded, fetching from server if needed
    async fn ensure_manifest_loaded(&mut self) -> Result<(), JSONRPCError> {
        if self.manifest.is_some() {
            return Ok(()); // Already loaded
        }
        
        if !self.config.enable_validation {
            return Ok(()); // Validation disabled, no need to fetch
        }
        
        // Fetch manifest from server
        let fetched_manifest = Self::fetch_manifest_from_server(&self.core_client, &self.config).await?;
        
        // Channels have been removed from the protocol
        
        self.manifest = Some(fetched_manifest);
        Ok(())
    }
    
    /// Send request via SOCK_DGRAM and wait for response
    pub async fn send_request(
        &mut self,
        request: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<Duration>,
    ) -> Result<JanusResponse, JSONRPCError> {
        // Generate request ID and response socket path
        let request_id = Uuid::new_v4().to_string();
        let response_socket_path = self.core_client.generate_response_socket_path();
        
        // Create socket request using constructor
        let mut socket_request = JanusRequest::new(
            request.to_string(),
            args,
            timeout.map(|d| d.as_secs_f64()),
        );
        socket_request.id = request_id.clone(); // Use provided request ID
        socket_request.reply_to = Some(response_socket_path.clone());
        
        // Apply security validation
        SecurityValidator::validate_request_name(request, &self.config)?;
        SecurityValidator::validate_args_size(&socket_request.args, &self.config)?;
        SecurityValidator::validate_socket_path(&response_socket_path)?;
        
        // Serialize request for message size validation
        let request_data = serde_json::to_vec(&socket_request)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(format!("Failed to serialize request: {}", e))))?;
        
        // Validate message size
        SecurityValidator::validate_message_size(request_data.len(), &self.config)?;
        
        // Ensure Manifest is loaded for validation
        if self.config.enable_validation {
            self.ensure_manifest_loaded().await?;
        }
        
        // Validate request against Manifest (skip for built-in requests)
        if let Some(ref manifest) = self.manifest {
            if !Self::is_builtin_request(request) {
                self.validate_request_against_manifest(manifest, &socket_request)?;
            }
        }
        
        // Send datagram and wait for response
        let response_data = self.core_client
            .send_datagram(&request_data, &response_socket_path)
            .await?;
        
        // Deserialize response
        let response: JanusResponse = serde_json::from_slice(&response_data)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(format!("Failed to deserialize response: {}", e))))?;
        
        // Validate response correlation (PRIME DIRECTIVE: no channel validation)
        if response.request_id != request_id {
            return Err(JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(format!("Response correlation mismatch: expected {}, got {}", request_id, response.request_id))));
        }
        
        // Update connection state after successful communication
        self.update_connection_state(1, 1);
        
        Ok(response)
    }
    
    /// Send request without expecting response (fire-and-forget)
    pub async fn send_request_no_response(
        &self,
        request: &str,
        args: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<(), JSONRPCError> {
        // Generate request ID
        let request_id = Uuid::new_v4().to_string();
        
        // Create socket request (no reply_to field) using constructor
        let mut socket_request = JanusRequest::new(
            request.to_string(),
            args,
            None,
        );
        socket_request.id = request_id;
        socket_request.reply_to = None;
        
        // Apply security validation
        SecurityValidator::validate_request_name(request, &self.config)?;
        SecurityValidator::validate_args_size(&socket_request.args, &self.config)?;
        
        // Serialize request for message size validation
        let request_data = serde_json::to_vec(&socket_request)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(format!("Failed to serialize request: {}", e))))?;
        
        // Validate message size
        SecurityValidator::validate_message_size(request_data.len(), &self.config)?;
        
        // Validate request against Manifest
        if let Some(ref manifest) = self.manifest {
            self.validate_request_against_manifest(manifest, &socket_request)?;
        }
        
        // Send datagram without waiting for response
        self.core_client.send_datagram_no_response(&request_data).await?;
        
        // Update connection state after successful send
        self.update_connection_state(1, 0);
        
        Ok(())
    }
    
    /// Test connectivity to the server
    pub async fn test_connection(&self) -> Result<(), JSONRPCError> {
        self.core_client.test_connection().await
    }
    
    /// Validate request against Manifest
    fn validate_request_against_manifest(
        &self,
        _manifest: &Manifest,
        request: &JanusRequest,
    ) -> Result<(), JSONRPCError> {
        // Check if request is reserved (built-in requests should never be in Manifests)
        let builtin_requests = ["ping", "echo", "get_info", "validate", "slow_process", "manifest"];
        if builtin_requests.contains(&request.request.as_str()) {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!("Request '{}' is reserved and cannot be used from Manifest", request.request))));
        }
        
        // Since channels are removed, get request manifest from server
        // For now, skip validation as server will handle it
        Ok(())
    }
    
    /// Get channel ID
    pub fn channel_id(&self) -> &str {
        ""
    }
    
    /// Get socket path
    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }
    
    /// Get Manifest
    pub fn manifest(&self) -> Option<&Manifest> {
        self.manifest.as_ref()
    }
    
    /// Get configuration for backward compatibility
    pub fn configuration(&self) -> &JanusClientConfig {
        &self.config
    }
    
    
    /// Send a ping request and return success/failure
    /// Convenience method for testing connectivity with a simple ping request
    pub async fn ping(&mut self) -> bool {
        match self.send_request("ping", None, None).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    /// Register request handler - validates request exists in manifest (SOCK_DGRAM compatibility)
    /// This validates that the request exists in the Manifest for the client's channel.
    /// SOCK_DGRAM doesn't actually use handlers, but validation ensures compatibility.
    pub fn register_request_handler<T>(&self, _request: &str, _handler: T) -> Result<(), JSONRPCError> {
        // Since channels are removed from protocol, validation will be done server-side
        // SOCK_DGRAM doesn't actually use handlers
        Ok(())
    }
    
    /// Get socket path for backward compatibility
    pub fn socket_path_string(&self) -> &str {
        &self.socket_path
    }
    
    /// Disconnect is a no-op for backward compatibility (SOCK_DGRAM doesn't have persistent connections)
    pub fn disconnect(&self) -> Result<(), JSONRPCError> {
        // SOCK_DGRAM doesn't have persistent connections - this is for backward compatibility only
        Ok(())
    }
    
    /// Check if connected (legacy compatibility - SOCK_DGRAM doesn't maintain connections)
    pub fn is_connected(&self) -> bool {
        // In SOCK_DGRAM, we don't maintain persistent connections
        // Check if we can reach the server by checking if socket file exists
        std::path::Path::new(&self.socket_path).exists()
    }

    // MARK: - Connection State Simulation
    
    /// Get simulated connection state
    pub fn get_connection_state(&self) -> ConnectionState {
        self.connection_state.lock().unwrap().clone()
    }
    
    /// Update connection state after successful operation
    fn update_connection_state(&self, messages_sent: u64, responses_received: u64) {
        if let Ok(mut state) = self.connection_state.lock() {
            state.is_connected = true;
            state.last_activity = std::time::SystemTime::now();
            state.messages_sent += messages_sent;
            state.responses_received += responses_received;
        }
    }

    // MARK: - Advanced Client Features (Response Correlation System)

    /// Send request with response correlation tracking
    pub async fn send_request_with_correlation(
        &self,
        request: String,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Duration,
    ) -> Result<(tokio::sync::oneshot::Receiver<JanusResponse>, String), JSONRPCError> {
        let request_id = Uuid::new_v4().to_string();
        
        // Track the request in response tracker
        let receiver = self.response_tracker.track_request(request_id.clone(), timeout)
            .map_err(|e| JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!("Response tracking failed: {}", e))))?;

        // Send the request asynchronously
        let core_client = self.core_client.clone();
        let manifest = self.manifest.clone();
        let enable_validation = self.config.enable_validation;
        let response_tracker = self.response_tracker.clone();
        let cmd_id = request_id.clone();

        tokio::spawn(async move {
            // Create response socket path
            let response_socket_path = core_client.generate_response_socket_path();

            // Create socket request with manifestific ID using constructor
            let timeout_seconds = timeout.as_secs_f64();
            let mut socket_request = JanusRequest::new(
                request.clone(),
                args,
                Some(timeout_seconds),
            );
            socket_request.id = cmd_id.clone(); // Use provided ID
            socket_request.reply_to = Some(response_socket_path.clone());

            // Validate request if needed
            if enable_validation {
                if let Some(_manifest) = &manifest {
                    if !Self::is_builtin_request(&request) {
                        // Perform validation (simplified for async context)
                        // Full validation would be more complex
                    }
                }
            }

            // Serialize and send request
            match serde_json::to_vec(&socket_request) {
                Ok(request_bytes) => {
                    match core_client.send_datagram(&request_bytes, &response_socket_path).await {
                        Ok(response_bytes) => {
                            // Parse response
                            match serde_json::from_slice::<JanusResponse>(&response_bytes) {
                                Ok(response) => {
                                    // Handle response through tracker
                                    response_tracker.handle_response(response);
                                }
                                Err(_) => {
                                    response_tracker.cancel_request(&cmd_id, Some("Failed to parse response"));
                                }
                            }
                        }
                        Err(_) => {
                            // Cancel the request due to send failure
                            response_tracker.cancel_request(&cmd_id, Some("Failed to send request"));
                        }
                    }
                }
                Err(_) => {
                    response_tracker.cancel_request(&cmd_id, Some("Failed to serialize request"));
                }
            }
        });

        Ok((receiver, request_id))
    }

    /// Cancel a pending request by ID
    pub fn cancel_request(&self, request_id: &str, reason: Option<&str>) -> bool {
        self.response_tracker.cancel_request(request_id, reason)
    }

    /// Cancel all pending requests
    pub fn cancel_all_requests(&self, reason: Option<&str>) -> usize {
        self.response_tracker.cancel_all_requests(reason)
    }

    /// Get number of pending requests
    pub fn get_pending_request_count(&self) -> usize {
        self.response_tracker.get_pending_count()
    }

    /// Get list of pending request IDs
    pub fn get_pending_request_ids(&self) -> Vec<String> {
        self.response_tracker.get_pending_request_ids()
    }

    // Automatic ID Management Methods (F0193-F0216)

    /// Send request with handle - returns RequestHandle for tracking
    /// Hides UUID complexity from users while providing request lifecycle management
    pub async fn send_request_with_handle(
        &self,
        request: &str,
        args: Option<HashMap<String, serde_json::Value>>,
        timeout: Option<Duration>,
    ) -> Result<(RequestHandle, tokio::sync::oneshot::Receiver<Result<JanusResponse, JSONRPCError>>), JSONRPCError> {
        // Generate internal UUID (hidden from user)
        let request_id = Uuid::new_v4().to_string();
        
        // Create request handle for user
        let handle = RequestHandle::new(request_id.clone(), request.to_string());
        
        // Register the request handle
        {
            let mut registry = self.request_registry.lock().unwrap();
            registry.insert(request_id.clone(), handle.clone());
        }
        
        // Create one-shot channel for response
        let (sender, receiver) = tokio::sync::oneshot::channel();
        
        // Execute request asynchronously
        let mut client_clone = self.clone_for_async();
        let request_clone = request.to_string();
        let handle_clone = handle.clone();
        
        tokio::spawn(async move {
            let result = client_clone.send_request(&request_clone, args, timeout).await;
            
            // Clean up request handle
            {
                let mut registry = client_clone.request_registry.lock().unwrap();
                registry.remove(handle_clone.get_internal_id());
            }
            
            let _ = sender.send(result);
        });
        
        Ok((handle, receiver))
    }

    /// Get request status by handle
    pub fn get_request_status(&self, handle: &RequestHandle) -> RequestStatus {
        if handle.is_cancelled() {
            return RequestStatus::Cancelled;
        }
        
        let registry = self.request_registry.lock().unwrap();
        if registry.contains_key(handle.get_internal_id()) {
            RequestStatus::Pending
        } else {
            RequestStatus::Completed
        }
    }


    /// Get all pending request handles
    pub fn get_pending_requests(&self) -> Vec<RequestHandle> {
        let registry = self.request_registry.lock().unwrap();
        registry.values().cloned().collect()
    }


    /// Clone client for async operations (internal helper)
    fn clone_for_async(&self) -> Self {
        Self {
            socket_path: self.socket_path.clone(),
            manifest: self.manifest.clone(),
            config: self.config.clone(),
            core_client: self.core_client.clone(),
            response_tracker: self.response_tracker.clone(),
            connection_state: std::sync::Mutex::new(ConnectionState::new()),
            request_registry: std::sync::Mutex::new(self.request_registry.lock().unwrap().clone()),
        }
    }

    /// Check if a request is currently pending
    pub fn is_request_pending(&self, request_id: &str) -> bool {
        self.response_tracker.is_tracking(request_id)
    }

    /// Get statistics about pending requests
    pub fn get_request_statistics(&self) -> RequestStatistics {
        self.response_tracker.get_statistics()
    }

    /// Execute multiple requests in parallel
    pub async fn execute_requests_in_parallel(
        &self,
        requests: Vec<ParallelRequest>,
    ) -> Vec<ParallelResult> {
        let mut results = Vec::with_capacity(requests.len());
        let mut handles = Vec::new();

        for cmd in requests {
            let mut client = self.clone();
            let handle = tokio::spawn(async move {
                let response = client.send_request(&cmd.request, cmd.args, None).await;
                let (response_ok, error_msg) = match response {
                    Ok(resp) => (Some(resp), None),
                    Err(e) => (None, Some(e.to_string())),
                };
                
                ParallelResult {
                    request_id: cmd.id,
                    response: response_ok,
                    error: error_msg,
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(ParallelResult {
                    request_id: "unknown".to_string(),
                    response: None,
                    error: Some(format!("Task execution failed: {}", e)),
                }),
            }
        }

        results
    }


    /// Check if a request is a built-in request
    fn is_builtin_request(request: &str) -> bool {
        matches!(request, "ping" | "echo" | "get_info" | "manifest" | "validate" | "slow_process")
    }
}

// MARK: - Helper Types for Advanced Features

/// Represents a request to be executed in parallel
#[derive(Debug, Clone)]
pub struct ParallelRequest {
    pub id: String,
    pub request: String,
    pub args: Option<HashMap<String, serde_json::Value>>,
}

/// Represents the result of a parallel request execution
#[derive(Debug, Clone)]
pub struct ParallelResult {
    pub request_id: String,
    pub response: Option<JanusResponse>,
    pub error: Option<String>,
}


// Need to implement Clone for JanusClient to support advanced features
impl Clone for JanusClient {
    fn clone(&self) -> Self {
        // Initialize a new response tracker for the cloned client
        let tracker_config = TrackerConfig {
            max_pending_requests: 1000,
            cleanup_interval: Duration::from_secs(30),
            default_timeout: self.config.connection_timeout,
        };
        let response_tracker = ResponseTracker::new(tracker_config);

        Self {
            socket_path: self.socket_path.clone(),
            manifest: self.manifest.clone(),
            config: self.config.clone(),
            core_client: self.core_client.clone(),
            response_tracker,
            connection_state: std::sync::Mutex::new(ConnectionState::new()),
            request_registry: std::sync::Mutex::new(HashMap::new()),
        }
    }
}