use crate::protocol::message_types::{JanusResponse};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::time::{interval, MissedTickBehavior};
use tokio::task::JoinHandle;

/// Represents a request awaiting response
#[derive(Debug)]
pub struct PendingRequest {
    pub resolve: oneshot::Sender<JanusResponse>,
    pub timestamp: Instant,
    pub timeout: Duration,
}

/// Configuration for the response tracker
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    pub max_pending_requests: usize,
    pub cleanup_interval: Duration,
    pub default_timeout: Duration,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            max_pending_requests: 1000,
            cleanup_interval: Duration::from_secs(30),
            default_timeout: Duration::from_secs(30),
        }
    }
}

/// Response tracker error types
#[derive(Debug, thiserror::Error)]
pub enum ResponseTrackerError {
    #[error("Too many pending requests: maximum {max} requests allowed")]
    PendingRequestsLimit { max: usize },
    
    #[error("Request already being tracked: {request_id}")]
    DuplicateRequestId { request_id: String },
    
    #[error("Request timeout: {request_id} timed out after {timeout:?}")]
    RequestTimeout { request_id: String, timeout: Duration },
    
    #[error("Request cancelled: {request_id} - {reason}")]
    RequestCancelled { request_id: String, reason: String },
    
    #[error("All requests cancelled: {reason}")]
    AllRequestsCancelled { reason: String },
    
    #[error("Request failed: {message}")]
    RequestFailed { message: String },
}

/// Statistics about pending requests
#[derive(Debug, Clone)]
pub struct RequestStatistics {
    pub pending_count: usize,
    pub average_age: f64,
    pub oldest_request: Option<RequestInfo>,
    pub newest_request: Option<RequestInfo>,
}

/// Information about a manifestific request
#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub id: String,
    pub age: f64,
}

/// Response tracker manages async response correlation and timeout handling
#[derive(Debug, Clone)]
pub struct ResponseTracker {
    pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
    config: TrackerConfig,
    cleanup_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ResponseTracker {
    /// Create a new response tracker
    pub fn new(config: TrackerConfig) -> Self {
        let pending_requests = Arc::new(Mutex::new(HashMap::new()));
        let cleanup_task = Self::start_cleanup_task(pending_requests.clone(), config.cleanup_interval);

        Self {
            pending_requests,
            config,
            cleanup_task: Arc::new(Mutex::new(Some(cleanup_task))),
        }
    }

    /// Track a request awaiting response
    pub fn track_request(
        &self,
        request_id: String,
        timeout_duration: Duration,
    ) -> Result<oneshot::Receiver<JanusResponse>, ResponseTrackerError> {
        let timeout = if timeout_duration.is_zero() {
            self.config.default_timeout
        } else {
            timeout_duration
        };

        let mut pending = self.pending_requests.lock().unwrap();

        // Check limits
        if pending.len() >= self.config.max_pending_requests {
            return Err(ResponseTrackerError::PendingRequestsLimit {
                max: self.config.max_pending_requests,
            });
        }

        // Check for duplicate tracking
        if pending.contains_key(&request_id) {
            return Err(ResponseTrackerError::DuplicateRequestId { request_id });
        }

        // Create pending request entry
        let (tx, rx) = oneshot::channel();
        let pending_request = PendingRequest {
            resolve: tx,
            timestamp: Instant::now(),
            timeout,
        };

        pending.insert(request_id.clone(), pending_request);

        // Set individual timeout
        let pending_requests = self.pending_requests.clone();
        let cmd_id = request_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            Self::handle_timeout(pending_requests, cmd_id, timeout).await;
        });

        Ok(rx)
    }

    /// Handle an incoming response
    pub fn handle_response(&self, response: JanusResponse) -> bool {
        let mut pending = self.pending_requests.lock().unwrap();
        
        if let Some(pending_request) = pending.remove(&response.request_id) {
            // Send response through the channel
            let _ = pending_request.resolve.send(response);
            true
        } else {
            // Response for unknown request (possibly timed out)
            false
        }
    }

    /// Cancel tracking for a request
    pub fn cancel_request(&self, request_id: &str, reason: Option<&str>) -> bool {
        let mut pending = self.pending_requests.lock().unwrap();
        
        if let Some(pending_request) = pending.remove(request_id) {
            let reason = reason.unwrap_or("Request cancelled");
            let _error = ResponseTrackerError::RequestCancelled {
                request_id: request_id.to_string(),
                reason: reason.to_string(),
            };
            
            // The receiver will get an error when the sender is dropped
            drop(pending_request);
            true
        } else {
            false
        }
    }

    /// Cancel all pending requests
    pub fn cancel_all_requests(&self, _reason: Option<&str>) -> usize {
        let mut pending = self.pending_requests.lock().unwrap();
        let count = pending.len();
        
        // Drop all pending requests, which will cause their receivers to get errors
        pending.clear();
        
        count
    }

    /// Get number of pending requests
    pub fn get_pending_count(&self) -> usize {
        let pending = self.pending_requests.lock().unwrap();
        pending.len()
    }

    /// Get list of pending request IDs
    pub fn get_pending_request_ids(&self) -> Vec<String> {
        let pending = self.pending_requests.lock().unwrap();
        pending.keys().cloned().collect()
    }

    /// Check if a request is being tracked
    pub fn is_tracking(&self, request_id: &str) -> bool {
        let pending = self.pending_requests.lock().unwrap();
        pending.contains_key(request_id)
    }

    /// Get statistics about pending requests
    pub fn get_statistics(&self) -> RequestStatistics {
        let pending = self.pending_requests.lock().unwrap();
        let now = Instant::now();

        if pending.is_empty() {
            return RequestStatistics {
                pending_count: 0,
                average_age: 0.0,
                oldest_request: None,
                newest_request: None,
            };
        }

        let mut total_age = 0.0;
        let mut oldest: Option<(String, f64)> = None;
        let mut newest: Option<(String, f64)> = None;

        for (id, request) in pending.iter() {
            let age = now.duration_since(request.timestamp).as_secs_f64();
            total_age += age;

            if oldest.is_none() || age > oldest.as_ref().unwrap().1 {
                oldest = Some((id.clone(), age));
            }

            if newest.is_none() || age < newest.as_ref().unwrap().1 {
                newest = Some((id.clone(), age));
            }
        }

        let average_age = total_age / pending.len() as f64;

        RequestStatistics {
            pending_count: pending.len(),
            average_age,
            oldest_request: oldest.map(|(id, age)| RequestInfo { id, age }),
            newest_request: newest.map(|(id, age)| RequestInfo { id, age }),
        }
    }

    /// Cleanup expired requests
    pub fn cleanup(&self) -> usize {
        let mut pending = self.pending_requests.lock().unwrap();
        let now = Instant::now();
        let mut expired_requests = Vec::new();

        for (request_id, request) in pending.iter() {
            let age = now.duration_since(request.timestamp);
            if age >= request.timeout {
                expired_requests.push(request_id.clone());
            }
        }

        let count = expired_requests.len();
        for request_id in expired_requests {
            pending.remove(&request_id);
        }

        count
    }

    /// Shutdown the tracker and cleanup resources
    pub async fn shutdown(&self) {
        if let Ok(mut cleanup_guard) = self.cleanup_task.lock() {
            if let Some(cleanup_task) = cleanup_guard.take() {
                cleanup_task.abort();
                let _ = cleanup_task.await;
            }
        }

        self.cancel_all_requests(Some("Tracker shutdown"));
    }

    /// Handle request timeout
    async fn handle_timeout(
        pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
        request_id: String,
        timeout_duration: Duration,
    ) {
        let mut pending = pending_requests.lock().unwrap();
        
        if let Some(pending_request) = pending.remove(&request_id) {
            // Check if the request actually timed out (not just finished normally)
            let age = Instant::now().duration_since(pending_request.timestamp);
            if age >= timeout_duration {
                // Drop the pending request, which will cause the receiver to get an error
                drop(pending_request);
            } else {
                // Request finished normally, put it back
                pending.insert(request_id, pending_request);
            }
        }
    }

    /// Start the cleanup task
    fn start_cleanup_task(
        pending_requests: Arc<Mutex<HashMap<String, PendingRequest>>>,
        cleanup_interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = interval(cleanup_interval);
            interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;
                
                let now = Instant::now();
                let mut expired_requests = Vec::new();

                {
                    let pending = pending_requests.lock().unwrap();
                    for (request_id, request) in pending.iter() {
                        let age = now.duration_since(request.timestamp);
                        if age >= request.timeout {
                            expired_requests.push(request_id.clone());
                        }
                    }
                }

                if !expired_requests.is_empty() {
                    let mut pending = pending_requests.lock().unwrap();
                    for request_id in expired_requests {
                        pending.remove(&request_id);
                    }
                }
            }
        })
    }
}

impl Drop for ResponseTracker {
    fn drop(&mut self) {
        if let Ok(mut cleanup_guard) = self.cleanup_task.lock() {
            if let Some(cleanup_task) = cleanup_guard.take() {
                cleanup_task.abort();
            }
        }
    }
}