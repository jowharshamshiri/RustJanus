use crate::protocol::message_types::{JanusResponse};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use tokio::time::{interval, MissedTickBehavior};
use tokio::task::JoinHandle;

/// Represents a command awaiting response
#[derive(Debug)]
pub struct PendingCommand {
    pub resolve: oneshot::Sender<JanusResponse>,
    pub timestamp: Instant,
    pub timeout: Duration,
}

/// Configuration for the response tracker
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    pub max_pending_commands: usize,
    pub cleanup_interval: Duration,
    pub default_timeout: Duration,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            max_pending_commands: 1000,
            cleanup_interval: Duration::from_secs(30),
            default_timeout: Duration::from_secs(30),
        }
    }
}

/// Response tracker error types
#[derive(Debug, thiserror::Error)]
pub enum ResponseTrackerError {
    #[error("Too many pending commands: maximum {max} commands allowed")]
    PendingCommandsLimit { max: usize },
    
    #[error("Command already being tracked: {command_id}")]
    DuplicateCommandId { command_id: String },
    
    #[error("Command timeout: {command_id} timed out after {timeout:?}")]
    CommandTimeout { command_id: String, timeout: Duration },
    
    #[error("Command cancelled: {command_id} - {reason}")]
    CommandCancelled { command_id: String, reason: String },
    
    #[error("All commands cancelled: {reason}")]
    AllCommandsCancelled { reason: String },
    
    #[error("Command failed: {message}")]
    CommandFailed { message: String },
}

/// Statistics about pending commands
#[derive(Debug, Clone)]
pub struct CommandStatistics {
    pub pending_count: usize,
    pub average_age: f64,
    pub oldest_command: Option<CommandInfo>,
    pub newest_command: Option<CommandInfo>,
}

/// Information about a specific command
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub id: String,
    pub age: f64,
}

/// Response tracker manages async response correlation and timeout handling
#[derive(Debug, Clone)]
pub struct ResponseTracker {
    pending_commands: Arc<Mutex<HashMap<String, PendingCommand>>>,
    config: TrackerConfig,
    cleanup_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ResponseTracker {
    /// Create a new response tracker
    pub fn new(config: TrackerConfig) -> Self {
        let pending_commands = Arc::new(Mutex::new(HashMap::new()));
        let cleanup_task = Self::start_cleanup_task(pending_commands.clone(), config.cleanup_interval);

        Self {
            pending_commands,
            config,
            cleanup_task: Arc::new(Mutex::new(Some(cleanup_task))),
        }
    }

    /// Track a command awaiting response
    pub fn track_command(
        &self,
        command_id: String,
        timeout_duration: Duration,
    ) -> Result<oneshot::Receiver<JanusResponse>, ResponseTrackerError> {
        let timeout = if timeout_duration.is_zero() {
            self.config.default_timeout
        } else {
            timeout_duration
        };

        let mut pending = self.pending_commands.lock().unwrap();

        // Check limits
        if pending.len() >= self.config.max_pending_commands {
            return Err(ResponseTrackerError::PendingCommandsLimit {
                max: self.config.max_pending_commands,
            });
        }

        // Check for duplicate tracking
        if pending.contains_key(&command_id) {
            return Err(ResponseTrackerError::DuplicateCommandId { command_id });
        }

        // Create pending command entry
        let (tx, rx) = oneshot::channel();
        let pending_command = PendingCommand {
            resolve: tx,
            timestamp: Instant::now(),
            timeout,
        };

        pending.insert(command_id.clone(), pending_command);

        // Set individual timeout
        let pending_commands = self.pending_commands.clone();
        let cmd_id = command_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(timeout).await;
            Self::handle_timeout(pending_commands, cmd_id, timeout).await;
        });

        Ok(rx)
    }

    /// Handle an incoming response
    pub fn handle_response(&self, response: JanusResponse) -> bool {
        let mut pending = self.pending_commands.lock().unwrap();
        
        if let Some(pending_command) = pending.remove(&response.commandId) {
            // Send response through the channel
            let _ = pending_command.resolve.send(response);
            true
        } else {
            // Response for unknown command (possibly timed out)
            false
        }
    }

    /// Cancel tracking for a command
    pub fn cancel_command(&self, command_id: &str, reason: Option<&str>) -> bool {
        let mut pending = self.pending_commands.lock().unwrap();
        
        if let Some(pending_command) = pending.remove(command_id) {
            let reason = reason.unwrap_or("Command cancelled");
            let _error = ResponseTrackerError::CommandCancelled {
                command_id: command_id.to_string(),
                reason: reason.to_string(),
            };
            
            // The receiver will get an error when the sender is dropped
            drop(pending_command);
            true
        } else {
            false
        }
    }

    /// Cancel all pending commands
    pub fn cancel_all_commands(&self, _reason: Option<&str>) -> usize {
        let mut pending = self.pending_commands.lock().unwrap();
        let count = pending.len();
        
        // Drop all pending commands, which will cause their receivers to get errors
        pending.clear();
        
        count
    }

    /// Get number of pending commands
    pub fn get_pending_count(&self) -> usize {
        let pending = self.pending_commands.lock().unwrap();
        pending.len()
    }

    /// Get list of pending command IDs
    pub fn get_pending_command_ids(&self) -> Vec<String> {
        let pending = self.pending_commands.lock().unwrap();
        pending.keys().cloned().collect()
    }

    /// Check if a command is being tracked
    pub fn is_tracking(&self, command_id: &str) -> bool {
        let pending = self.pending_commands.lock().unwrap();
        pending.contains_key(command_id)
    }

    /// Get statistics about pending commands
    pub fn get_statistics(&self) -> CommandStatistics {
        let pending = self.pending_commands.lock().unwrap();
        let now = Instant::now();

        if pending.is_empty() {
            return CommandStatistics {
                pending_count: 0,
                average_age: 0.0,
                oldest_command: None,
                newest_command: None,
            };
        }

        let mut total_age = 0.0;
        let mut oldest: Option<(String, f64)> = None;
        let mut newest: Option<(String, f64)> = None;

        for (id, command) in pending.iter() {
            let age = now.duration_since(command.timestamp).as_secs_f64();
            total_age += age;

            if oldest.is_none() || age > oldest.as_ref().unwrap().1 {
                oldest = Some((id.clone(), age));
            }

            if newest.is_none() || age < newest.as_ref().unwrap().1 {
                newest = Some((id.clone(), age));
            }
        }

        let average_age = total_age / pending.len() as f64;

        CommandStatistics {
            pending_count: pending.len(),
            average_age,
            oldest_command: oldest.map(|(id, age)| CommandInfo { id, age }),
            newest_command: newest.map(|(id, age)| CommandInfo { id, age }),
        }
    }

    /// Cleanup expired commands
    pub fn cleanup(&self) -> usize {
        let mut pending = self.pending_commands.lock().unwrap();
        let now = Instant::now();
        let mut expired_commands = Vec::new();

        for (command_id, command) in pending.iter() {
            let age = now.duration_since(command.timestamp);
            if age >= command.timeout {
                expired_commands.push(command_id.clone());
            }
        }

        let count = expired_commands.len();
        for command_id in expired_commands {
            pending.remove(&command_id);
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

        self.cancel_all_commands(Some("Tracker shutdown"));
    }

    /// Handle command timeout
    async fn handle_timeout(
        pending_commands: Arc<Mutex<HashMap<String, PendingCommand>>>,
        command_id: String,
        timeout_duration: Duration,
    ) {
        let mut pending = pending_commands.lock().unwrap();
        
        if let Some(pending_command) = pending.remove(&command_id) {
            // Check if the command actually timed out (not just finished normally)
            let age = Instant::now().duration_since(pending_command.timestamp);
            if age >= timeout_duration {
                // Drop the pending command, which will cause the receiver to get an error
                drop(pending_command);
            } else {
                // Command finished normally, put it back
                pending.insert(command_id, pending_command);
            }
        }
    }

    /// Start the cleanup task
    fn start_cleanup_task(
        pending_commands: Arc<Mutex<HashMap<String, PendingCommand>>>,
        cleanup_interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = interval(cleanup_interval);
            interval_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;
                
                let now = Instant::now();
                let mut expired_commands = Vec::new();

                {
                    let pending = pending_commands.lock().unwrap();
                    for (command_id, command) in pending.iter() {
                        let age = now.duration_since(command.timestamp);
                        if age >= command.timeout {
                            expired_commands.push(command_id.clone());
                        }
                    }
                }

                if !expired_commands.is_empty() {
                    let mut pending = pending_commands.lock().unwrap();
                    for command_id in expired_commands {
                        pending.remove(&command_id);
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