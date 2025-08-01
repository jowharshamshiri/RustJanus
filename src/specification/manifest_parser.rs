use crate::error::{JSONRPCError, JSONRPCErrorCode};
use crate::specification::Manifest;
use log::{debug, error, info, warn};
use tokio::fs;

/// Manifest parser for JSON and YAML formats (exact SwiftJanus parity)
pub struct ManifestParser;

impl ManifestParser {
    /// Parse Manifest from JSON string
    pub fn from_json(json_str: &str) -> Result<Manifest, JSONRPCError> {
        Self::from_json_with_context(json_str, None)
    }

    /// Parse Manifest from JSON string with file context
    pub fn from_json_with_context(
        json_str: &str,
        file_path: Option<&str>,
    ) -> Result<Manifest, JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        debug!(
            "Attempting to parse Manifest from JSON{} ({} bytes)",
            context,
            json_str.len()
        );

        // Validate JSON string is not empty
        if json_str.trim().is_empty() {
            error!("Manifest JSON string is empty{}", context);
            return Err(JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(format!("JSON parsing error{}: input string is empty", context))));
        }

        // Log the first part of JSON for debugging (truncated to avoid sensitive data exposure)
        let preview = if json_str.len() > 200 {
            format!("{}...", &json_str[..200])
        } else {
            json_str.to_string()
        };
        debug!("JSON content preview{}: {}", context, preview);

        match serde_json::from_str::<Manifest>(json_str) {
            Ok(manifest) => {
                info!("Successfully parsed Manifest from JSON{}", context);
                debug!("Parsed Manifest version: {}", manifest.version);
                debug!("Number of channels: {}", manifest.channels.len());
                Ok(manifest)
            }
            Err(e) => {
                error!(
                    "Failed to parse Manifest from JSON{}: {}",
                    context, e
                );

                // Provide more detailed error information based on error type
                let detailed_error = match e.classify() {
                    serde_json::error::Category::Io => {
                        format!("JSON parsing error{} - I/O issue: {}", context, e)
                    }
                    serde_json::error::Category::Syntax => {
                        let line = e.line();
                        let column = e.column();
                        error!(
                            "JSON syntax error{} at line {}, column {}",
                            context, line, column
                        );
                        format!(
                            "JSON parsing error{} - Syntax error at line {}, column {}: {}",
                            context, line, column, e
                        )
                    }
                    serde_json::error::Category::Data => {
                        error!("JSON data structure error{}: {}", context, e);
                        format!(
                            "JSON parsing error{} - Invalid data structure: {}",
                            context, e
                        )
                    }
                    serde_json::error::Category::Eof => {
                        error!("JSON parsing error{} - Unexpected end of file", context);
                        format!(
                            "JSON parsing error{} - Unexpected end of file: {}",
                            context, e
                        )
                    }
                };

                Err(JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(detailed_error)))
            }
        }
    }

    /// Parse Manifest from YAML string
    #[cfg(feature = "yaml-support")]
    pub fn from_yaml(yaml_str: &str) -> Result<Manifest, JSONRPCError> {
        Self::from_yaml_with_context(yaml_str, None)
    }

    /// Parse Manifest from YAML string with file context
    #[cfg(feature = "yaml-support")]
    pub fn from_yaml_with_context(
        yaml_str: &str,
        file_path: Option<&str>,
    ) -> Result<Manifest, JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        debug!(
            "Attempting to parse Manifest from YAML{} ({} bytes)",
            context,
            yaml_str.len()
        );

        // Validate YAML string is not empty
        if yaml_str.trim().is_empty() {
            error!("Manifest YAML string is empty{}", context);
            return Err(JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(format!("YAML parsing error{}: input string is empty", context))));
        }

        // Log the first part of YAML for debugging (truncated to avoid sensitive data exposure)
        let preview = if yaml_str.len() > 200 {
            format!("{}...", &yaml_str[..200])
        } else {
            yaml_str.to_string()
        };
        debug!("YAML content preview{}: {}", context, preview);

        match serde_yaml::from_str::<Manifest>(yaml_str) {
            Ok(manifest) => {
                info!("Successfully parsed Manifest from YAML{}", context);
                debug!("Parsed Manifest version: {}", manifest.version);
                debug!("Number of channels: {}", manifest.channels.len());
                Ok(manifest)
            }
            Err(e) => {
                error!(
                    "Failed to parse Manifest from YAML{}: {}",
                    context, e
                );

                // Provide detailed error information for YAML parsing
                let detailed_error = if let Some(location) = e.location() {
                    let line = location.line();
                    let column = location.column();
                    error!(
                        "YAML syntax error{} at line {}, column {}",
                        context, line, column
                    );
                    format!(
                        "YAML parsing error{} - Syntax error at line {}, column {}: {}",
                        context, line, column, e
                    )
                } else {
                    format!("YAML parsing error{}: {}", context, e)
                };

                Err(JSONRPCError::new(JSONRPCErrorCode::ParseError, Some(detailed_error)))
            }
        }
    }

    /// Parse Manifest from file (auto-detect format based on extension)
    pub async fn from_file(path: &str) -> Result<Manifest, JSONRPCError> {
        info!("Loading Manifest from file: {}", path);

        // Validate file path
        if path.trim().is_empty() {
            error!("Manifest file path is empty");
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some("File path cannot be empty".to_string())));
        }

        // Check if file exists and log file information
        match fs::metadata(path).await {
            Ok(metadata) => {
                debug!("File found: {} ({} bytes)", path, metadata.len());
                if metadata.len() == 0 {
                    warn!("Manifest file is empty: {}", path);
                }
                if metadata.len() > 10_000_000 {
                    // 10MB limit
                    warn!(
                        "Manifest file is very large ({} bytes): {}",
                        metadata.len(),
                        path
                    );
                }
            }
            Err(e) => {
                error!("Cannot access Manifest file '{}': {}", path, e);
                return Err(JSONRPCError::new(JSONRPCErrorCode::ResourceNotFound, Some(format!("Failed to access file {}: {}", path, e))));
            }
        }

        // Read file content
        let content = match fs::read_to_string(path).await {
            Ok(content) => {
                debug!("Successfully read {} bytes from {}", content.len(), path);
                content
            }
            Err(e) => {
                error!("Failed to read Manifest file '{}': {}", path, e);
                return Err(JSONRPCError::new(JSONRPCErrorCode::ResourceNotFound, Some(format!("Failed to read file {}: {}", path, e))));
            }
        };

        // Determine format and parse with file context
        let result = if path.ends_with(".yaml") || path.ends_with(".yml") {
            info!("Detected YAML format for file: {}", path);
            #[cfg(feature = "yaml-support")]
            {
                Self::from_yaml_with_context(&content, Some(path))
            }
            #[cfg(not(feature = "yaml-support"))]
            {
                error!("YAML support not enabled for file: {}", path);
                Err(JSONRPCError::new(JSONRPCErrorCode::ConfigurationError, Some(format!("YAML support not enabled (file: {}). Enable 'yaml-support' feature.", path))))
            }
        } else if path.ends_with(".json") {
            info!("Detected JSON format for file: {}", path);
            Self::from_json_with_context(&content, Some(path))
        } else {
            info!(
                "Unknown file extension for {}, defaulting to JSON format",
                path
            );
            Self::from_json_with_context(&content, Some(path))
        };

        match &result {
            Ok(manifest) => {
                info!("Successfully loaded Manifest from {}", path);
                debug!(
                    "Loaded spec version: {} with {} channels",
                    manifest.version,
                    manifest.channels.len()
                );
            }
            Err(e) => {
                error!("Failed to parse Manifest from {}: {}", path, e);
            }
        }

        result
    }

    /// Serialize Manifest to JSON string
    pub fn to_json(manifest: &Manifest) -> Result<String, JSONRPCError> {
        debug!("Serializing Manifest to JSON");
        serde_json::to_string_pretty(manifest).map_err(|e| {
            error!("Failed to serialize Manifest to JSON: {}", e);
            JSONRPCError::new(JSONRPCErrorCode::InternalError, Some(format!("JSON serialization error: {}", e)))
        })
    }

    /// Serialize Manifest to YAML string
    #[cfg(feature = "yaml-support")]
    pub fn to_yaml(manifest: &Manifest) -> Result<String, JanusError> {
        debug!("Serializing Manifest to YAML");
        serde_yaml::to_string(manifest).map_err(|e| {
            error!("Failed to serialize Manifest to YAML: {}", e);
            JSONRPCErrorCode::EncodingFailed(format!("YAML serialization error: {}", e))
        })
    }

    /// Write Manifest to file (format based on extension)
    pub async fn to_file(manifest: &Manifest, path: &str) -> Result<(), JSONRPCError> {
        let content = if path.ends_with(".yaml") || path.ends_with(".yml") {
            #[cfg(feature = "yaml-support")]
            {
                Self::to_yaml(manifest)?
            }
            #[cfg(not(feature = "yaml-support"))]
            {
                return Err(JSONRPCError::new(JSONRPCErrorCode::ConfigurationError, Some("YAML support not enabled. Enable 'yaml-support' feature.".to_string())));
            }
        } else {
            Self::to_json(manifest)?
        };

        fs::write(path, content).await.map_err(|e| {
            JSONRPCError::new(JSONRPCErrorCode::ResourceNotFound, Some(format!("Failed to write file {}: {}", path, e)))
        })?;

        Ok(())
    }

    /// Validate Manifest structure and content
    pub fn validate(manifest: &Manifest) -> Result<(), JSONRPCError> {
        Self::validate_with_context(manifest, None)
    }

    /// Validate Manifest structure and content with file context
    pub fn validate_with_context(
        manifest: &Manifest,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        info!("Starting Manifest validation{}", context);
        debug!("Validating Manifest version: {}", manifest.version);

        // Validate version
        if manifest.version.is_empty() {
            error!(
                "Manifest validation failed{}: version is required",
                context
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Manifest version is required{}",
                context
            ))));
        }

        // Validate version format (semantic versioning)
        if !Self::is_valid_version(&manifest.version) {
            error!(
                "Manifest validation failed{}: invalid version format '{}'",
                context, manifest.version
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Invalid version format: {}{}",
                manifest.version, context
            ))));
        }
        debug!("✓ Version format is valid: {}", manifest.version);

        // Validate channels
        if manifest.channels.is_empty() {
            error!(
                "Manifest validation failed{}: no channels defined",
                context
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Manifest must define at least one channel{}",
                context
            ))));
        }
        debug!("Validating {} channels", manifest.channels.len());

        for (channel_name, channel_spec) in &manifest.channels {
            debug!("Validating channel: {}", channel_name);
            if let Err(e) = Self::validate_channel(channel_name, channel_spec, file_path) {
                error!(
                    "Channel validation failed for '{}'{}: {}",
                    channel_name, context, e
                );
                return Err(e);
            }
            debug!("✓ Channel '{}' is valid", channel_name);
        }

        // Validate models if present
        if let Some(models) = &manifest.models {
            debug!("Validating {} models", models.len());
            for (model_name, model_spec) in models {
                debug!("Validating model: {}", model_name);
                if let Err(e) = Self::validate_model(model_name, model_spec, manifest, file_path) {
                    error!(
                        "Model validation failed for '{}'{}: {}",
                        model_name, context, e
                    );
                    return Err(e);
                }
                debug!("✓ Model '{}' is valid", model_name);
            }
        } else {
            debug!("No models defined in Manifest");
        }

        info!(
            "✓ Manifest validation completed successfully{}",
            context
        );
        info!(
            "Validated{}: version {}, {} channels, {} models",
            context,
            manifest.version,
            manifest.channels.len(),
            manifest.models.as_ref().map_or(0, |m| m.len())
        );

        Ok(())
    }

    /// Load and validate Manifest from file in one step
    pub async fn load_and_validate(path: &str) -> Result<Manifest, JSONRPCError> {
        info!("Loading and validating Manifest from: {}", path);

        // Load the specification
        let manifest = Self::from_file(path).await?;

        // Validate the loaded specification
        Self::validate_with_context(&manifest, Some(path))?;

        info!(
            "Successfully loaded and validated Manifest from: {}",
            path
        );
        Ok(manifest)
    }

    /// Load and validate Manifest from JSON string in one step
    pub fn load_and_validate_json(json_str: &str) -> Result<Manifest, JSONRPCError> {
        Self::load_and_validate_json_with_context(json_str, None)
    }

    /// Load and validate Manifest from JSON string with file context
    pub fn load_and_validate_json_with_context(
        json_str: &str,
        file_path: Option<&str>,
    ) -> Result<Manifest, JSONRPCError> {
        let context = file_path
            .map(|p| format!(" from file: {}", p))
            .unwrap_or_default();
        info!(
            "Loading and validating Manifest from JSON string{}",
            context
        );

        // Parse the JSON with context
        let manifest = Self::from_json_with_context(json_str, file_path)?;

        // Validate the parsed specification with context
        Self::validate_with_context(&manifest, file_path)?;

        info!(
            "Successfully loaded and validated Manifest from JSON{}",
            context
        );
        Ok(manifest)
    }

    /// Load and validate Manifest from YAML string in one step
    #[cfg(feature = "yaml-support")]
    pub fn load_and_validate_yaml(yaml_str: &str) -> Result<Manifest, JSONRPCError> {
        Self::load_and_validate_yaml_with_context(yaml_str, None)
    }

    /// Load and validate Manifest from YAML string with file context
    #[cfg(feature = "yaml-support")]
    pub fn load_and_validate_yaml_with_context(
        yaml_str: &str,
        file_path: Option<&str>,
    ) -> Result<Manifest, JSONRPCError> {
        let context = file_path
            .map(|p| format!(" from file: {}", p))
            .unwrap_or_default();
        info!(
            "Loading and validating Manifest from YAML string{}",
            context
        );

        // Parse the YAML with context
        let manifest = Self::from_yaml_with_context(yaml_str, file_path)?;

        // Validate the parsed specification with context
        Self::validate_with_context(&manifest, file_path)?;

        info!(
            "Successfully loaded and validated Manifest from YAML{}",
            context
        );
        Ok(manifest)
    }

    /// Get a summary of validation errors for diagnostics
    pub fn get_validation_summary(manifest: &Manifest) -> String {
        let mut summary = Vec::new();

        // Check version
        if manifest.version.is_empty() {
            summary.push("• Missing version".to_string());
        } else if !Self::is_valid_version(&manifest.version) {
            summary.push(format!("• Invalid version format: {}", manifest.version));
        }

        // Check channels
        if manifest.channels.is_empty() {
            summary.push("• No channels defined".to_string());
        } else {
            for (channel_name, channel_spec) in &manifest.channels {
                if channel_name.is_empty() {
                    summary.push("• Empty channel name found".to_string());
                }
                if channel_spec.description.is_empty() {
                    summary.push(format!("• Channel '{}' missing description", channel_name));
                }
                if channel_spec.commands.is_empty() {
                    summary.push(format!("• Channel '{}' has no commands", channel_name));
                }
            }
        }

        // Check models if present
        if let Some(models) = &manifest.models {
            for (model_name, model_spec) in models {
                if model_name.is_empty() {
                    summary.push("• Empty model name found".to_string());
                }
                if model_spec.properties.is_empty() {
                    summary.push(format!("• Model '{}' has no properties", model_name));
                }
            }
        }

        if summary.is_empty() {
            "Manifest appears to be valid".to_string()
        } else {
            format!("Manifest issues found:\n{}", summary.join("\n"))
        }
    }

    /// Validate semantic version format
    fn is_valid_version(version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        parts.iter().all(|part| part.parse::<u32>().is_ok())
    }

    /// Validate channel specification
    fn validate_channel(
        channel_name: &str,
        channel_spec: &crate::specification::ChannelSpec,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        debug!(
            "Validating channel '{}' with {} commands{}",
            channel_name,
            channel_spec.commands.len(),
            context
        );

        // Channel name validation
        if channel_name.is_empty() {
            error!("Channel validation failed{}: name cannot be empty", context);
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidRequest, Some(format!(
                "Channel name cannot be empty{}",
                context
            ))));
        }

        // Channel name format validation (alphanumeric, hyphens, underscores)
        if !channel_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            error!(
                "Channel validation failed{}: invalid name format '{}'",
                context, channel_name
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidRequest, Some(format!(
                "Invalid channel name format: {}{}",
                channel_name, context
            ))));
        }

        // Description validation
        if channel_spec.description.is_empty() {
            error!(
                "Channel validation failed{}: '{}' must have a description",
                context, channel_name
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidRequest, Some(format!(
                "Channel '{}' must have a description{}",
                channel_name, context
            ))));
        }

        // Commands validation
        if channel_spec.commands.is_empty() {
            error!(
                "Channel validation failed{}: '{}' must define at least one command",
                context, channel_name
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidRequest, Some(format!(
                "Channel '{}' must define at least one command{}",
                channel_name, context
            ))));
        }

        debug!(
            "Validating {} commands in channel '{}'",
            channel_spec.commands.len(),
            channel_name
        );
        for (command_name, command_spec) in &channel_spec.commands {
            debug!(
                "Validating command '{}' in channel '{}'",
                command_name, channel_name
            );
            if let Err(e) =
                Self::validate_command_spec(channel_name, command_name, command_spec, file_path)
            {
                error!(
                    "Command validation failed for '{}' in channel '{}'{}: {}",
                    command_name, channel_name, context, e
                );
                return Err(e);
            }
        }

        Ok(())
    }

    /// Validate command specification
    fn validate_command_spec(
        channel_name: &str,
        command_name: &str,
        command_spec: &crate::specification::CommandSpec,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Command name validation
        if command_name.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MethodNotFound, Some(format!("Command name cannot be empty{}", context))));
        }

        // Command name format validation
        if !command_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MethodNotFound, Some(format!("Invalid command name format: {}{}", command_name, context))));
        }

        // Validate against reserved command names - built-in commands cannot be redefined
        let reserved_commands = ["ping", "echo", "get_info", "validate", "slow_process", "spec"];
        if reserved_commands.contains(&command_name) {
            error!(
                "Command validation failed{}: '{}' is a reserved built-in command",
                context, command_name
            );
            return Err(JSONRPCError::new(JSONRPCErrorCode::MethodNotFound, Some(format!("Command '{}' is reserved and cannot be defined in Manifest{}. Reserved commands: {}", command_name, context, reserved_commands.join(", ")))));
        }
        debug!("✓ Command '{}' is not reserved", command_name);

        // Description validation
        if command_spec.description.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::MethodNotFound, Some(format!("Command '{}' in channel '{}' must have a description{}", command_name, channel_name, context))));
        }

        // Validate arguments
        for (arg_name, arg_spec) in &command_spec.args {
            Self::validate_argument_spec(arg_name, arg_spec, file_path)?;
        }

        // Validate response spec
        Self::validate_response_spec(&command_spec.response, file_path)?;

        // Validate error codes if present
        if let Some(error_codes) = &command_spec.error_codes {
            for (error_name, error_spec) in error_codes {
                Self::validate_error_code_spec(error_name, error_spec, file_path)?;
            }
        }

        Ok(())
    }

    /// Validate argument specification
    fn validate_argument_spec(
        arg_name: &str,
        arg_spec: &crate::specification::ArgumentSpec,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Argument name validation
        if arg_name.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Argument name cannot be empty{}", context))));
        }

        // Type validation
        let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
        if !valid_types.contains(&arg_spec.r#type.as_str()) {
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Invalid argument type: {}{}", arg_spec.r#type, context))));
        }

        // Validation constraint validation
        if let Some(validation) = &arg_spec.validation {
            Self::validate_validation_spec(arg_name, validation, file_path)?;
        }

        // Default value type validation
        if let Some(default_value) = &arg_spec.default_value {
            Self::validate_value_type(arg_name, default_value, &arg_spec.r#type, file_path)?;
        }

        Ok(())
    }

    /// Validate validation constraints
    fn validate_validation_spec(
        arg_name: &str,
        validation_spec: &crate::specification::ValidationSpec,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Range validation
        if let (Some(min), Some(max)) = (validation_spec.minimum, validation_spec.maximum) {
            if min > max {
                return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Minimum value cannot be greater than maximum value{}", context))));
            }
        }

        // Length validation
        if let (Some(min_len), Some(max_len)) =
            (validation_spec.min_length, validation_spec.max_length)
        {
            if min_len > max_len {
                return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Minimum length cannot be greater than maximum length{}", context))));
            }
        }

        // Pattern validation
        if let Some(pattern) = &validation_spec.pattern {
            regex::Regex::new(pattern).map_err(|e| {
                JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Invalid regex pattern: {}{}", e, context)))
            })?;
        }

        // Enum validation
        if let Some(enum_values) = &validation_spec.r#enum {
            if enum_values.is_empty() {
                return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Enum values cannot be empty{}", context))));
            }
        }

        Ok(())
    }

    /// Validate response specification
    fn validate_response_spec(
        response_spec: &crate::specification::ResponseSpec,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Response type validation
        let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
        if !valid_types.contains(&response_spec.r#type.as_str()) {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Invalid response type: {}{}",
                response_spec.r#type, context
            ))));
        }

        // Validate properties if object type
        if response_spec.r#type == "object" {
            if let Some(properties) = &response_spec.properties {
                for (prop_name, prop_spec) in properties {
                    Self::validate_argument_spec(prop_name, prop_spec, file_path)?;
                }
            }
        }

        Ok(())
    }

    /// Validate error code specification
    fn validate_error_code_spec(
        error_name: &str,
        error_spec: &crate::specification::ErrorCodeSpec,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        if error_name.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Error code name cannot be empty{}",
                context
            ))));
        }

        if error_spec.message.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Error code '{}' must have a message{}",
                error_name, context
            ))));
        }

        // Validate HTTP status code range
        if !(100..=599).contains(&error_spec.code) {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Invalid HTTP status code: {}{}",
                error_spec.code, context
            ))));
        }

        Ok(())
    }

    /// Validate model specification
    fn validate_model(
        model_name: &str,
        model_spec: &crate::specification::ModelSpec,
        _manifest: &Manifest,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Model name validation
        if model_name.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                "Model name cannot be empty{}",
                context
            ))));
        }

        // Validate properties
        for (prop_name, prop_spec) in &model_spec.properties {
            Self::validate_argument_spec(prop_name, prop_spec, file_path)?;
        }

        // Validate required fields exist
        if let Some(required_fields) = &model_spec.required {
            for required_field in required_fields {
                if !model_spec.properties.contains_key(required_field) {
                    return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                        "Required field '{}' not found in model '{}'{}",
                        required_field, model_name, context
                    ))));
                }
            }
        }

        Ok(())
    }

    /// Validate value matches declared type
    fn validate_value_type(
        arg_name: &str,
        value: &serde_json::Value,
        expected_type: &str,
        file_path: Option<&str>,
    ) -> Result<(), JSONRPCError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        let matches = match expected_type {
            "string" => value.is_string(),
            "integer" => value.is_i64(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            _ => false,
        };

        if !matches {
            return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidParams, Some(format!("Value type mismatch: expected {}, got {}{}", expected_type, Self::get_value_type_name(value), context))));
        }

        Ok(())
    }

    /// Parse multiple Manifest files and merge them
    pub async fn parse_multiple_files(file_paths: &[String]) -> Result<Manifest, JSONRPCError> {
        if file_paths.is_empty() {
            return Err(JSONRPCError::new(JSONRPCErrorCode::ResourceNotFound, Some("No files provided".to_string())));
        }

        info!("Parsing {} Manifest files", file_paths.len());
        
        // Parse first file as base
        let mut base_spec = Self::from_file(&file_paths[0]).await?;
        info!("Base specification loaded from: {}", file_paths[0]);
        
        // Merge additional files
        for file_path in &file_paths[1..] {
            info!("Merging specification from: {}", file_path);
            let additional_spec = Self::from_file(file_path).await?;
            Self::merge_specifications(&mut base_spec, &additional_spec)?;
        }
        
        // Validate merged specification
        Self::validate(&base_spec)?;
        
        info!("Successfully merged {} specification files", file_paths.len());
        Ok(base_spec)
    }

    /// Merge two Manifests
    pub fn merge_specifications(base: &mut Manifest, additional: &Manifest) -> Result<(), JSONRPCError> {
        info!("Merging Manifests");
        
        // Merge channels
        for (channel_id, channel_spec) in &additional.channels {
            if base.channels.contains_key(channel_id) {
                return Err(JSONRPCError::new(JSONRPCErrorCode::InvalidRequest, Some(format!(
                    "Channel '{}' already exists in base specification", 
                    channel_id
                ))));
            }
            base.channels.insert(channel_id.clone(), channel_spec.clone());
        }
        
        // Merge models if present
        if let Some(additional_models) = &additional.models {
            let base_models = base.models.get_or_insert_with(std::collections::HashMap::new);
            
            for (model_name, model_spec) in additional_models {
                if base_models.contains_key(model_name) {
                    return Err(JSONRPCError::new(JSONRPCErrorCode::ValidationFailed, Some(format!(
                        "Model '{}' already exists in base specification", 
                        model_name
                    ))));
                }
                base_models.insert(model_name.clone(), model_spec.clone());
            }
        }
        
        info!("Specifications merged successfully");
        Ok(())
    }

    /// Get JSON value type name
    fn get_value_type_name(value: &serde_json::Value) -> &'static str {
        match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        }
    }
}

// Static interface methods for convenience (matching Go/Swift patterns)
impl ManifestParser {
    /// Static method for parsing JSON
    pub fn parse_json(json_str: &str) -> Result<Manifest, JSONRPCError> {
        Self::from_json(json_str)
    }

    /// Static method for parsing YAML
    #[cfg(feature = "yaml-support")]
    pub fn parse_yaml(yaml_str: &str) -> Result<Manifest, JSONRPCError> {
        Self::from_yaml(yaml_str)
    }

    /// Static method for parsing from file
    pub async fn parse_from_file(path: &str) -> Result<Manifest, JSONRPCError> {
        Self::from_file(path).await
    }

    /// Static method for validation
    pub fn validate_specification(manifest: &Manifest) -> Result<(), JSONRPCError> {
        Self::validate(manifest)
    }

    /// Static method for JSON serialization
    pub fn serialize_to_json(manifest: &Manifest) -> Result<String, JSONRPCError> {
        Self::to_json(manifest)
    }

    /// Static method for YAML serialization
    #[cfg(feature = "yaml-support")]
    pub fn serialize_to_yaml(manifest: &Manifest) -> Result<String, JanusError> {
        Self::to_yaml(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specification::{
        ArgumentSpec, ChannelSpec, CommandSpec, ResponseSpec, ValidationSpec,
    };

    fn create_test_manifest() -> Manifest {
        let mut manifest = Manifest::new("1.0.0".to_string());

        let mut channel = ChannelSpec::new("Test channel".to_string());

        let mut command = CommandSpec::new(
            "Test command".to_string(),
            ResponseSpec::new("object".to_string()),
        );

        let arg = ArgumentSpec::new("string".to_string())
            .required()
            .with_validation(ValidationSpec::new().with_length_range(Some(1), Some(100)));

        command.add_argument("test_arg".to_string(), arg);
        channel.add_command("test_cmd".to_string(), command);
        manifest.add_channel("test_channel".to_string(), channel);

        manifest
    }

    #[test]
    fn test_json_serialization() {
        let manifest = create_test_manifest();

        let json_str = ManifestParser::to_json(&manifest).unwrap();
        assert!(!json_str.is_empty());

        let parsed_spec = ManifestParser::from_json(&json_str).unwrap();
        assert_eq!(parsed_spec.version, manifest.version);
        assert_eq!(parsed_spec.channels.len(), manifest.channels.len());
    }

    #[cfg(feature = "yaml-support")]
    #[test]
    fn test_yaml_serialization() {
        let manifest = create_test_manifest();

        let yaml_str = ManifestParser::to_yaml(&manifest).unwrap();
        assert!(!yaml_str.is_empty());

        let parsed_spec = ManifestParser::from_yaml(&yaml_str).unwrap();
        assert_eq!(parsed_spec.version, manifest.version);
        assert_eq!(parsed_spec.channels.len(), manifest.channels.len());
    }

    #[test]
    fn test_validation_success() {
        let manifest = create_test_manifest();
        let result = ManifestParser::validate(&manifest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_empty_version() {
        let mut manifest = create_test_manifest();
        manifest.version = String::new();

        let result = ManifestParser::validate(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_version() {
        let mut manifest = create_test_manifest();
        manifest.version = "invalid".to_string();

        let result = ManifestParser::validate(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_empty_channels() {
        let mut manifest = create_test_manifest();
        manifest.channels.clear();

        let result = ManifestParser::validate(&manifest);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_validation() {
        assert!(ManifestParser::is_valid_version("1.0.0"));
        assert!(ManifestParser::is_valid_version("10.20.30"));
        assert!(!ManifestParser::is_valid_version("1.0"));
        assert!(!ManifestParser::is_valid_version("1.0.0.0"));
        assert!(!ManifestParser::is_valid_version("v1.0.0"));
        assert!(!ManifestParser::is_valid_version("1.0.0-beta"));
    }

    #[test]
    fn test_malformed_json() {
        let malformed_json = r#"{"version": "1.0.0", "channels": }"#;
        let result = ManifestParser::from_json(malformed_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_value_type_validation() {
        let string_value = serde_json::Value::String("test".to_string());
        let number_value = serde_json::Value::Number(serde_json::Number::from(42));

        assert!(
            ManifestParser::validate_value_type("test", &string_value, "string", None).is_ok()
        );
        assert!(
            ManifestParser::validate_value_type("test", &number_value, "number", None).is_ok()
        );
        assert!(
            ManifestParser::validate_value_type("test", &string_value, "number", None).is_err()
        );
        assert!(
            ManifestParser::validate_value_type("test", &number_value, "string", None).is_err()
        );
    }

    #[test]
    fn test_specification_merging() {
        let mut base_spec = create_test_manifest();
        
        // Create additional specification
        let mut additional_spec = Manifest::new("1.0.0".to_string());
        let mut additional_channel = ChannelSpec::new("Additional Channel".to_string());
        let additional_command = CommandSpec::new(
            "Additional Command".to_string(),
            ResponseSpec::new("object".to_string()),
        );
        additional_channel.add_command("additional_cmd".to_string(), additional_command);
        additional_spec.add_channel("additional_channel".to_string(), additional_channel);
        
        // Merge specifications
        let result = ManifestParser::merge_specifications(&mut base_spec, &additional_spec);
        assert!(result.is_ok());
        
        // Verify merge results
        assert_eq!(base_spec.channels.len(), 2);
        assert!(base_spec.channels.contains_key("test_channel"));
        assert!(base_spec.channels.contains_key("additional_channel"));
    }

    #[test]
    fn test_specification_merging_conflict() {
        let mut base_spec = create_test_manifest();
        
        // Create conflicting specification (same channel name)
        let mut conflicting_spec = Manifest::new("1.0.0".to_string());
        let mut conflicting_channel = ChannelSpec::new("Conflicting Channel".to_string());
        let conflicting_command = CommandSpec::new(
            "Conflicting Command".to_string(), 
            ResponseSpec::new("object".to_string()),
        );
        conflicting_channel.add_command("conflicting_cmd".to_string(), conflicting_command);
        conflicting_spec.add_channel("test_channel".to_string(), conflicting_channel); // Same name as base
        
        // Merge should fail due to conflict
        let result = ManifestParser::merge_specifications(&mut base_spec, &conflicting_spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_static_methods() {
        let manifest = create_test_manifest();
        
        // Test static JSON serialization
        let json_str = ManifestParser::serialize_to_json(&manifest).unwrap();
        assert!(!json_str.is_empty());
        
        // Test static JSON parsing
        let parsed_spec = ManifestParser::parse_json(&json_str).unwrap();
        assert_eq!(parsed_spec.version, manifest.version);
        
        // Test static validation
        let result = ManifestParser::validate_specification(&parsed_spec);
        assert!(result.is_ok());
    }

    #[cfg(feature = "yaml-support")]
    #[test]
    fn test_static_yaml_methods() {
        let manifest = create_test_manifest();
        
        // Test static YAML serialization
        let yaml_str = ManifestParser::serialize_to_yaml(&manifest).unwrap();
        assert!(!yaml_str.is_empty());
        
        // Test static YAML parsing
        let parsed_spec = ManifestParser::parse_yaml(&yaml_str).unwrap();
        assert_eq!(parsed_spec.version, manifest.version);
    }
}
