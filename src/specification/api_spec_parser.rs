use crate::error::JanusError;
use crate::specification::ApiSpecification;
use log::{debug, error, info, warn};
use tokio::fs;

/// API specification parser for JSON and YAML formats (exact SwiftJanus parity)
pub struct ApiSpecificationParser;

impl ApiSpecificationParser {
    /// Parse API specification from JSON string
    pub fn from_json(json_str: &str) -> Result<ApiSpecification, JanusError> {
        Self::from_json_with_context(json_str, None)
    }

    /// Parse API specification from JSON string with file context
    pub fn from_json_with_context(
        json_str: &str,
        file_path: Option<&str>,
    ) -> Result<ApiSpecification, JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        debug!(
            "Attempting to parse API specification from JSON{} ({} bytes)",
            context,
            json_str.len()
        );

        // Validate JSON string is not empty
        if json_str.trim().is_empty() {
            error!("API specification JSON string is empty{}", context);
            return Err(JanusError::DecodingFailed(format!(
                "JSON parsing error{}: input string is empty",
                context
            )));
        }

        // Log the first part of JSON for debugging (truncated to avoid sensitive data exposure)
        let preview = if json_str.len() > 200 {
            format!("{}...", &json_str[..200])
        } else {
            json_str.to_string()
        };
        debug!("JSON content preview{}: {}", context, preview);

        match serde_json::from_str::<ApiSpecification>(json_str) {
            Ok(api_spec) => {
                info!("Successfully parsed API specification from JSON{}", context);
                debug!("Parsed API spec version: {}", api_spec.version);
                debug!("Number of channels: {}", api_spec.channels.len());
                Ok(api_spec)
            }
            Err(e) => {
                error!(
                    "Failed to parse API specification from JSON{}: {}",
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

                Err(JanusError::DecodingFailed(detailed_error))
            }
        }
    }

    /// Parse API specification from YAML string
    #[cfg(feature = "yaml-support")]
    pub fn from_yaml(yaml_str: &str) -> Result<ApiSpecification, JanusError> {
        Self::from_yaml_with_context(yaml_str, None)
    }

    /// Parse API specification from YAML string with file context
    #[cfg(feature = "yaml-support")]
    pub fn from_yaml_with_context(
        yaml_str: &str,
        file_path: Option<&str>,
    ) -> Result<ApiSpecification, JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        debug!(
            "Attempting to parse API specification from YAML{} ({} bytes)",
            context,
            yaml_str.len()
        );

        // Validate YAML string is not empty
        if yaml_str.trim().is_empty() {
            error!("API specification YAML string is empty{}", context);
            return Err(JanusError::DecodingFailed(format!(
                "YAML parsing error{}: input string is empty",
                context
            )));
        }

        // Log the first part of YAML for debugging (truncated to avoid sensitive data exposure)
        let preview = if yaml_str.len() > 200 {
            format!("{}...", &yaml_str[..200])
        } else {
            yaml_str.to_string()
        };
        debug!("YAML content preview{}: {}", context, preview);

        match serde_yaml::from_str::<ApiSpecification>(yaml_str) {
            Ok(api_spec) => {
                info!("Successfully parsed API specification from YAML{}", context);
                debug!("Parsed API spec version: {}", api_spec.version);
                debug!("Number of channels: {}", api_spec.channels.len());
                Ok(api_spec)
            }
            Err(e) => {
                error!(
                    "Failed to parse API specification from YAML{}: {}",
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

                Err(JanusError::DecodingFailed(detailed_error))
            }
        }
    }

    /// Parse API specification from file (auto-detect format based on extension)
    pub async fn from_file(path: &str) -> Result<ApiSpecification, JanusError> {
        info!("Loading API specification from file: {}", path);

        // Validate file path
        if path.trim().is_empty() {
            error!("API specification file path is empty");
            return Err(JanusError::IoError(
                "File path cannot be empty".to_string(),
            ));
        }

        // Check if file exists and log file information
        match fs::metadata(path).await {
            Ok(metadata) => {
                debug!("File found: {} ({} bytes)", path, metadata.len());
                if metadata.len() == 0 {
                    warn!("API specification file is empty: {}", path);
                }
                if metadata.len() > 10_000_000 {
                    // 10MB limit
                    warn!(
                        "API specification file is very large ({} bytes): {}",
                        metadata.len(),
                        path
                    );
                }
            }
            Err(e) => {
                error!("Cannot access API specification file '{}': {}", path, e);
                return Err(JanusError::IoError(format!(
                    "Failed to access file {}: {}",
                    path, e
                )));
            }
        }

        // Read file content
        let content = match fs::read_to_string(path).await {
            Ok(content) => {
                debug!("Successfully read {} bytes from {}", content.len(), path);
                content
            }
            Err(e) => {
                error!("Failed to read API specification file '{}': {}", path, e);
                return Err(JanusError::IoError(format!(
                    "Failed to read file {}: {}",
                    path, e
                )));
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
                Err(JanusError::DecodingFailed(format!(
                    "YAML support not enabled (file: {}). Enable 'yaml-support' feature.",
                    path
                )))
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
            Ok(api_spec) => {
                info!("Successfully loaded API specification from {}", path);
                debug!(
                    "Loaded spec version: {} with {} channels",
                    api_spec.version,
                    api_spec.channels.len()
                );
            }
            Err(e) => {
                error!("Failed to parse API specification from {}: {}", path, e);
            }
        }

        result
    }

    /// Serialize API specification to JSON string
    pub fn to_json(api_spec: &ApiSpecification) -> Result<String, JanusError> {
        debug!("Serializing API specification to JSON");
        serde_json::to_string_pretty(api_spec).map_err(|e| {
            error!("Failed to serialize API specification to JSON: {}", e);
            JanusError::EncodingFailed(format!("JSON serialization error: {}", e))
        })
    }

    /// Serialize API specification to YAML string
    #[cfg(feature = "yaml-support")]
    pub fn to_yaml(api_spec: &ApiSpecification) -> Result<String, JanusError> {
        debug!("Serializing API specification to YAML");
        serde_yaml::to_string(api_spec).map_err(|e| {
            error!("Failed to serialize API specification to YAML: {}", e);
            JanusError::EncodingFailed(format!("YAML serialization error: {}", e))
        })
    }

    /// Write API specification to file (format based on extension)
    pub async fn to_file(api_spec: &ApiSpecification, path: &str) -> Result<(), JanusError> {
        let content = if path.ends_with(".yaml") || path.ends_with(".yml") {
            #[cfg(feature = "yaml-support")]
            {
                Self::to_yaml(api_spec)?
            }
            #[cfg(not(feature = "yaml-support"))]
            {
                return Err(JanusError::EncodingFailed(
                    "YAML support not enabled. Enable 'yaml-support' feature.".to_string(),
                ));
            }
        } else {
            Self::to_json(api_spec)?
        };

        fs::write(path, content).await.map_err(|e| {
            JanusError::IoError(format!("Failed to write file {}: {}", path, e))
        })?;

        Ok(())
    }

    /// Validate API specification structure and content
    pub fn validate(api_spec: &ApiSpecification) -> Result<(), JanusError> {
        Self::validate_with_context(api_spec, None)
    }

    /// Validate API specification structure and content with file context
    pub fn validate_with_context(
        api_spec: &ApiSpecification,
        file_path: Option<&str>,
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        info!("Starting API specification validation{}", context);
        debug!("Validating API spec version: {}", api_spec.version);

        // Validate version
        if api_spec.version.is_empty() {
            error!(
                "API specification validation failed{}: version is required",
                context
            );
            return Err(JanusError::MalformedData(format!(
                "API specification version is required{}",
                context
            )));
        }

        // Validate version format (semantic versioning)
        if !Self::is_valid_version(&api_spec.version) {
            error!(
                "API specification validation failed{}: invalid version format '{}'",
                context, api_spec.version
            );
            return Err(JanusError::MalformedData(format!(
                "Invalid version format: {}{}",
                api_spec.version, context
            )));
        }
        debug!("✓ Version format is valid: {}", api_spec.version);

        // Validate channels
        if api_spec.channels.is_empty() {
            error!(
                "API specification validation failed{}: no channels defined",
                context
            );
            return Err(JanusError::MalformedData(format!(
                "API specification must define at least one channel{}",
                context
            )));
        }
        debug!("Validating {} channels", api_spec.channels.len());

        for (channel_name, channel_spec) in &api_spec.channels {
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
        if let Some(models) = &api_spec.models {
            debug!("Validating {} models", models.len());
            for (model_name, model_spec) in models {
                debug!("Validating model: {}", model_name);
                if let Err(e) = Self::validate_model(model_name, model_spec, api_spec, file_path) {
                    error!(
                        "Model validation failed for '{}'{}: {}",
                        model_name, context, e
                    );
                    return Err(e);
                }
                debug!("✓ Model '{}' is valid", model_name);
            }
        } else {
            debug!("No models defined in API specification");
        }

        info!(
            "✓ API specification validation completed successfully{}",
            context
        );
        info!(
            "Validated{}: version {}, {} channels, {} models",
            context,
            api_spec.version,
            api_spec.channels.len(),
            api_spec.models.as_ref().map_or(0, |m| m.len())
        );

        Ok(())
    }

    /// Load and validate API specification from file in one step
    pub async fn load_and_validate(path: &str) -> Result<ApiSpecification, JanusError> {
        info!("Loading and validating API specification from: {}", path);

        // Load the specification
        let api_spec = Self::from_file(path).await?;

        // Validate the loaded specification
        Self::validate_with_context(&api_spec, Some(path))?;

        info!(
            "Successfully loaded and validated API specification from: {}",
            path
        );
        Ok(api_spec)
    }

    /// Load and validate API specification from JSON string in one step
    pub fn load_and_validate_json(json_str: &str) -> Result<ApiSpecification, JanusError> {
        Self::load_and_validate_json_with_context(json_str, None)
    }

    /// Load and validate API specification from JSON string with file context
    pub fn load_and_validate_json_with_context(
        json_str: &str,
        file_path: Option<&str>,
    ) -> Result<ApiSpecification, JanusError> {
        let context = file_path
            .map(|p| format!(" from file: {}", p))
            .unwrap_or_default();
        info!(
            "Loading and validating API specification from JSON string{}",
            context
        );

        // Parse the JSON with context
        let api_spec = Self::from_json_with_context(json_str, file_path)?;

        // Validate the parsed specification with context
        Self::validate_with_context(&api_spec, file_path)?;

        info!(
            "Successfully loaded and validated API specification from JSON{}",
            context
        );
        Ok(api_spec)
    }

    /// Load and validate API specification from YAML string in one step
    #[cfg(feature = "yaml-support")]
    pub fn load_and_validate_yaml(yaml_str: &str) -> Result<ApiSpecification, JanusError> {
        Self::load_and_validate_yaml_with_context(yaml_str, None)
    }

    /// Load and validate API specification from YAML string with file context
    #[cfg(feature = "yaml-support")]
    pub fn load_and_validate_yaml_with_context(
        yaml_str: &str,
        file_path: Option<&str>,
    ) -> Result<ApiSpecification, JanusError> {
        let context = file_path
            .map(|p| format!(" from file: {}", p))
            .unwrap_or_default();
        info!(
            "Loading and validating API specification from YAML string{}",
            context
        );

        // Parse the YAML with context
        let api_spec = Self::from_yaml_with_context(yaml_str, file_path)?;

        // Validate the parsed specification with context
        Self::validate_with_context(&api_spec, file_path)?;

        info!(
            "Successfully loaded and validated API specification from YAML{}",
            context
        );
        Ok(api_spec)
    }

    /// Get a summary of validation errors for diagnostics
    pub fn get_validation_summary(api_spec: &ApiSpecification) -> String {
        let mut summary = Vec::new();

        // Check version
        if api_spec.version.is_empty() {
            summary.push("• Missing version".to_string());
        } else if !Self::is_valid_version(&api_spec.version) {
            summary.push(format!("• Invalid version format: {}", api_spec.version));
        }

        // Check channels
        if api_spec.channels.is_empty() {
            summary.push("• No channels defined".to_string());
        } else {
            for (channel_name, channel_spec) in &api_spec.channels {
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
        if let Some(models) = &api_spec.models {
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
            "API specification appears to be valid".to_string()
        } else {
            format!("API specification issues found:\n{}", summary.join("\n"))
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
    ) -> Result<(), JanusError> {
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
            return Err(JanusError::InvalidChannel(format!(
                "Channel name cannot be empty{}",
                context
            )));
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
            return Err(JanusError::InvalidChannel(format!(
                "Invalid channel name format: {}{}",
                channel_name, context
            )));
        }

        // Description validation
        if channel_spec.description.is_empty() {
            error!(
                "Channel validation failed{}: '{}' must have a description",
                context, channel_name
            );
            return Err(JanusError::InvalidChannel(format!(
                "Channel '{}' must have a description{}",
                channel_name, context
            )));
        }

        // Commands validation
        if channel_spec.commands.is_empty() {
            error!(
                "Channel validation failed{}: '{}' must define at least one command",
                context, channel_name
            );
            return Err(JanusError::InvalidChannel(format!(
                "Channel '{}' must define at least one command{}",
                channel_name, context
            )));
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
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Command name validation
        if command_name.is_empty() {
            return Err(JanusError::UnknownCommand(format!(
                "Command name cannot be empty{}",
                context
            )));
        }

        // Command name format validation
        if !command_name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(JanusError::UnknownCommand(format!(
                "Invalid command name format: {}{}",
                command_name, context
            )));
        }

        // Description validation
        if command_spec.description.is_empty() {
            return Err(JanusError::UnknownCommand(format!(
                "Command '{}' in channel '{}' must have a description{}",
                command_name, channel_name, context
            )));
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
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Argument name validation
        if arg_name.is_empty() {
            return Err(JanusError::InvalidArgument(
                "argument".to_string(),
                format!("Argument name cannot be empty{}", context),
            ));
        }

        // Type validation
        let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
        if !valid_types.contains(&arg_spec.r#type.as_str()) {
            return Err(JanusError::InvalidArgument(
                arg_name.to_string(),
                format!("Invalid argument type: {}{}", arg_spec.r#type, context),
            ));
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
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Range validation
        if let (Some(min), Some(max)) = (validation_spec.minimum, validation_spec.maximum) {
            if min > max {
                return Err(JanusError::InvalidArgument(
                    arg_name.to_string(),
                    format!(
                        "Minimum value cannot be greater than maximum value{}",
                        context
                    ),
                ));
            }
        }

        // Length validation
        if let (Some(min_len), Some(max_len)) =
            (validation_spec.min_length, validation_spec.max_length)
        {
            if min_len > max_len {
                return Err(JanusError::InvalidArgument(
                    arg_name.to_string(),
                    format!(
                        "Minimum length cannot be greater than maximum length{}",
                        context
                    ),
                ));
            }
        }

        // Pattern validation
        if let Some(pattern) = &validation_spec.pattern {
            regex::Regex::new(pattern).map_err(|e| {
                JanusError::InvalidArgument(
                    arg_name.to_string(),
                    format!("Invalid regex pattern: {}{}", e, context),
                )
            })?;
        }

        // Enum validation
        if let Some(enum_values) = &validation_spec.r#enum {
            if enum_values.is_empty() {
                return Err(JanusError::InvalidArgument(
                    arg_name.to_string(),
                    format!("Enum values cannot be empty{}", context),
                ));
            }
        }

        Ok(())
    }

    /// Validate response specification
    fn validate_response_spec(
        response_spec: &crate::specification::ResponseSpec,
        file_path: Option<&str>,
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Response type validation
        let valid_types = ["string", "integer", "number", "boolean", "array", "object"];
        if !valid_types.contains(&response_spec.r#type.as_str()) {
            return Err(JanusError::MalformedData(format!(
                "Invalid response type: {}{}",
                response_spec.r#type, context
            )));
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
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        if error_name.is_empty() {
            return Err(JanusError::MalformedData(format!(
                "Error code name cannot be empty{}",
                context
            )));
        }

        if error_spec.message.is_empty() {
            return Err(JanusError::MalformedData(format!(
                "Error code '{}' must have a message{}",
                error_name, context
            )));
        }

        // Validate HTTP status code range
        if !(100..=599).contains(&error_spec.code) {
            return Err(JanusError::MalformedData(format!(
                "Invalid HTTP status code: {}{}",
                error_spec.code, context
            )));
        }

        Ok(())
    }

    /// Validate model specification
    fn validate_model(
        model_name: &str,
        model_spec: &crate::specification::ModelSpec,
        _api_spec: &ApiSpecification,
        file_path: Option<&str>,
    ) -> Result<(), JanusError> {
        let context = file_path
            .map(|p| format!(" (file: {})", p))
            .unwrap_or_default();
        // Model name validation
        if model_name.is_empty() {
            return Err(JanusError::MalformedData(format!(
                "Model name cannot be empty{}",
                context
            )));
        }

        // Validate properties
        for (prop_name, prop_spec) in &model_spec.properties {
            Self::validate_argument_spec(prop_name, prop_spec, file_path)?;
        }

        // Validate required fields exist
        if let Some(required_fields) = &model_spec.required {
            for required_field in required_fields {
                if !model_spec.properties.contains_key(required_field) {
                    return Err(JanusError::MalformedData(format!(
                        "Required field '{}' not found in model '{}'{}",
                        required_field, model_name, context
                    )));
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
    ) -> Result<(), JanusError> {
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
            return Err(JanusError::InvalidArgument(
                arg_name.to_string(),
                format!(
                    "Value type mismatch: expected {}, got {}{}",
                    expected_type,
                    Self::get_value_type_name(value),
                    context
                ),
            ));
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specification::{
        ArgumentSpec, ChannelSpec, CommandSpec, ResponseSpec, ValidationSpec,
    };

    fn create_test_api_spec() -> ApiSpecification {
        let mut api_spec = ApiSpecification::new("1.0.0".to_string());

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
        api_spec.add_channel("test_channel".to_string(), channel);

        api_spec
    }

    #[test]
    fn test_json_serialization() {
        let api_spec = create_test_api_spec();

        let json_str = ApiSpecificationParser::to_json(&api_spec).unwrap();
        assert!(!json_str.is_empty());

        let parsed_spec = ApiSpecificationParser::from_json(&json_str).unwrap();
        assert_eq!(parsed_spec.version, api_spec.version);
        assert_eq!(parsed_spec.channels.len(), api_spec.channels.len());
    }

    #[cfg(feature = "yaml-support")]
    #[test]
    fn test_yaml_serialization() {
        let api_spec = create_test_api_spec();

        let yaml_str = ApiSpecificationParser::to_yaml(&api_spec).unwrap();
        assert!(!yaml_str.is_empty());

        let parsed_spec = ApiSpecificationParser::from_yaml(&yaml_str).unwrap();
        assert_eq!(parsed_spec.version, api_spec.version);
        assert_eq!(parsed_spec.channels.len(), api_spec.channels.len());
    }

    #[test]
    fn test_validation_success() {
        let api_spec = create_test_api_spec();
        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_empty_version() {
        let mut api_spec = create_test_api_spec();
        api_spec.version = String::new();

        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_invalid_version() {
        let mut api_spec = create_test_api_spec();
        api_spec.version = "invalid".to_string();

        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_empty_channels() {
        let mut api_spec = create_test_api_spec();
        api_spec.channels.clear();

        let result = ApiSpecificationParser::validate(&api_spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_validation() {
        assert!(ApiSpecificationParser::is_valid_version("1.0.0"));
        assert!(ApiSpecificationParser::is_valid_version("10.20.30"));
        assert!(!ApiSpecificationParser::is_valid_version("1.0"));
        assert!(!ApiSpecificationParser::is_valid_version("1.0.0.0"));
        assert!(!ApiSpecificationParser::is_valid_version("v1.0.0"));
        assert!(!ApiSpecificationParser::is_valid_version("1.0.0-beta"));
    }

    #[test]
    fn test_malformed_json() {
        let malformed_json = r#"{"version": "1.0.0", "channels": }"#;
        let result = ApiSpecificationParser::from_json(malformed_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_value_type_validation() {
        let string_value = serde_json::Value::String("test".to_string());
        let number_value = serde_json::Value::Number(serde_json::Number::from(42));

        assert!(
            ApiSpecificationParser::validate_value_type("test", &string_value, "string", None).is_ok()
        );
        assert!(
            ApiSpecificationParser::validate_value_type("test", &number_value, "number", None).is_ok()
        );
        assert!(
            ApiSpecificationParser::validate_value_type("test", &string_value, "number", None).is_err()
        );
        assert!(
            ApiSpecificationParser::validate_value_type("test", &number_value, "string", None).is_err()
        );
    }
}
