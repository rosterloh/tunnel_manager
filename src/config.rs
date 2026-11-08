use serde::{Deserialize, Serialize, de::Error};
use std::env;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to load configuration file: {0}")]
    FileLoad(#[from] std::io::Error),

    #[error("Failed to parse configuration: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Invalid service type: {service}")]
    InvalidService { service: String },

    #[error("Missing required configuration: {field}")]
    MissingRequired { field: String },
}

/// Supported tunnel service types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ServiceType {
    #[serde(rename = "ssh")]
    SSH,
    #[serde(rename = "gort")]
    GORT,
    #[serde(rename = "http")]
    HTTP,
    #[serde(rename = "custom")]
    Custom(String),
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::SSH => write!(f, "SSH"),
            ServiceType::GORT => write!(f, "GORT"),
            ServiceType::HTTP => write!(f, "HTTP"),
            ServiceType::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl std::str::FromStr for ServiceType {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "SSH" => Ok(ServiceType::SSH),
            "GORT" => Ok(ServiceType::GORT),
            "HTTP" => Ok(ServiceType::HTTP),
            _ => Ok(ServiceType::Custom(s.to_string())),
        }
    }
}

/// AWS configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    /// AWS profile name
    #[serde(default = "default_aws_profile")]
    pub profile: String,

    /// AWS region
    #[serde(default = "default_aws_region")]
    pub region: String,

    /// Timeout for AWS operations (in seconds)
    #[serde(default = "default_aws_timeout")]
    pub timeout_seconds: u64,

    /// Maximum retry attempts for AWS operations
    #[serde(default = "default_aws_max_retries")]
    pub max_retries: u32,
}

/// LocalProxy configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProxyConfig {
    /// Path to the localproxy executable
    #[serde(default = "default_localproxy_path")]
    pub path: String,

    /// Bind address for localproxy
    #[serde(default = "default_localproxy_bind")]
    pub bind_address: String,

    /// Working directory for localproxy
    #[serde(default = "default_localproxy_workdir")]
    pub working_directory: String,

    /// Default services to enable
    #[serde(default = "default_services")]
    pub default_services: Vec<ServiceType>,

    /// Service port mappings
    #[serde(default = "default_service_ports")]
    pub service_ports: std::collections::HashMap<ServiceType, u16>,
}

/// UI configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Application window title
    #[serde(default = "default_window_title")]
    pub window_title: String,

    /// Window width
    #[serde(default = "default_window_width")]
    pub window_width: f32,

    /// Window height
    #[serde(default = "default_window_height")]
    pub window_height: f32,

    /// Default theme (dark/light)
    #[serde(default = "default_theme")]
    pub theme: String,

    /// Connection timeout for UI operations (in seconds)
    #[serde(default = "default_ui_timeout")]
    pub connection_timeout: u64,
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// AWS-related configuration
    #[serde(default)]
    pub aws: AwsConfig,

    /// LocalProxy-related configuration
    #[serde(default)]
    pub localproxy: LocalProxyConfig,

    /// UI-related configuration
    #[serde(default)]
    pub ui: UiConfig,

    /// Enable debug logging
    #[serde(default = "default_debug")]
    pub debug: bool,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

// Default value functions
fn default_aws_profile() -> String {
    env::var("AWS_PROFILE").unwrap_or_else(|_| "iotmgmt_prod".to_string())
}

fn default_aws_region() -> String {
    env::var("AWS_REGION").unwrap_or_else(|_| "eu-west-1".to_string())
}

fn default_aws_timeout() -> u64 {
    env::var("AWS_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

fn default_aws_max_retries() -> u32 {
    env::var("AWS_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3)
}

fn default_localproxy_path() -> String {
    env::var("LOCALPROXY_PATH").unwrap_or_else(|_| "localproxy".to_string())
}

fn default_localproxy_bind() -> String {
    env::var("LOCALPROXY_BIND").unwrap_or_else(|_| "0.0.0.0".to_string())
}

fn default_localproxy_workdir() -> String {
    env::var("LOCALPROXY_WORKDIR").unwrap_or_else(|_| "assets".to_string())
}

fn default_services() -> Vec<ServiceType> {
    vec![ServiceType::SSH, ServiceType::GORT]
}

fn default_service_ports() -> std::collections::HashMap<ServiceType, u16> {
    let mut ports = std::collections::HashMap::new();
    ports.insert(ServiceType::SSH, 2222);
    ports.insert(ServiceType::GORT, 5555);
    ports.insert(ServiceType::HTTP, 8080);
    ports
}

fn default_window_title() -> String {
    env::var("WINDOW_TITLE").unwrap_or_else(|_| "Gardin Tunnel Manager".to_string())
}

fn default_window_width() -> f32 {
    env::var("WINDOW_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(430.0)
}

fn default_window_height() -> f32 {
    env::var("WINDOW_HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120.0)
}

fn default_theme() -> String {
    env::var("UI_THEME").unwrap_or_else(|_| "dark".to_string())
}

fn default_ui_timeout() -> u64 {
    env::var("UI_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

fn default_debug() -> bool {
    env::var("DEBUG")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false)
}

fn default_log_level() -> String {
    env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            profile: default_aws_profile(),
            region: default_aws_region(),
            timeout_seconds: default_aws_timeout(),
            max_retries: default_aws_max_retries(),
        }
    }
}

impl Default for LocalProxyConfig {
    fn default() -> Self {
        Self {
            path: default_localproxy_path(),
            bind_address: default_localproxy_bind(),
            working_directory: default_localproxy_workdir(),
            default_services: default_services(),
            service_ports: default_service_ports(),
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            window_title: default_window_title(),
            window_width: default_window_width(),
            window_height: default_window_height(),
            theme: default_theme(),
            connection_timeout: default_ui_timeout(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aws: AwsConfig::default(),
            localproxy: LocalProxyConfig::default(),
            ui: UiConfig::default(),
            debug: default_debug(),
            log_level: default_log_level(),
        }
    }
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), ConfigError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Parse(toml::de::Error::custom(e.to_string())))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration with fallback chain: file -> environment -> defaults
    pub fn load() -> Self {
        // Try to load from default config file locations
        let config_paths = [
            "tunnel-manager.toml",
            "config/tunnel-manager.toml",
            "~/.config/tunnel-manager/config.toml",
            "/etc/tunnel-manager/config.toml",
        ];

        for path in &config_paths {
            if let Ok(config) = Self::from_file(path) {
                return config;
            }
        }

        // Fallback to environment variables and defaults
        Self::default()
    }

    /// Validate configuration settings
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate AWS region format
        if self.aws.region.is_empty() {
            return Err(ConfigError::MissingRequired {
                field: "aws.region".to_string(),
            });
        }

        // Validate AWS profile
        if self.aws.profile.is_empty() {
            return Err(ConfigError::MissingRequired {
                field: "aws.profile".to_string(),
            });
        }

        // Validate localproxy path
        if self.localproxy.path.is_empty() {
            return Err(ConfigError::MissingRequired {
                field: "localproxy.path".to_string(),
            });
        }

        // Validate UI dimensions
        if self.ui.window_width <= 0.0 || self.ui.window_height <= 0.0 {
            return Err(ConfigError::MissingRequired {
                field: "ui.window_width/height must be positive".to_string(),
            });
        }

        Ok(())
    }

    /// Get service port mapping string for localproxy
    pub fn get_service_mapping(&self) -> String {
        self.localproxy
            .service_ports
            .iter()
            .map(|(service, port)| format!("{}={}", service, port))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Get list of service names for AWS tunnel configuration
    pub fn get_service_names(&self) -> Vec<String> {
        self.localproxy
            .default_services
            .iter()
            .map(|s| s.to_string())
            .collect()
    }
}
