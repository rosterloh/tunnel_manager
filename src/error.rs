use thiserror::Error;
use aws_sdk_iotsecuretunneling::error::SdkError;
use std::io;

/// Custom error types for the tunnel manager application
#[derive(Error, Debug)]
pub enum TunnelError {
    #[error("AWS configuration error: {message}")]
    AwsConfig { message: String },

    #[error("AWS authentication failed: {message}")]
    AwsAuth { message: String },

    #[error("Tunnel operation failed: {message}")]
    TunnelOperation { message: String },

    #[error("Tunnel not found for device: {device_id}")]
    TunnelNotFound { device_id: String },

    #[error("Process execution failed: {message}")]
    ProcessExecution { message: String },

    #[error("Invalid device ID: {device_id}")]
    InvalidDeviceId { device_id: String },

    #[error("Connection failed: {message}")]
    Connection { message: String },

    #[error("Token rotation failed for tunnel {tunnel_id}: {message}")]
    TokenRotation { tunnel_id: String, message: String },

    #[error("LocalProxy startup failed: {message}")]
    LocalProxyStartup { message: String },

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("AWS SDK error: {0}")]
    AwsSdk(String),
}

impl TunnelError {
    /// Create a new AWS configuration error
    pub fn aws_config(message: impl Into<String>) -> Self {
        Self::AwsConfig {
            message: message.into(),
        }
    }

    /// Create a new AWS authentication error
    pub fn aws_auth(message: impl Into<String>) -> Self {
        Self::AwsAuth {
            message: message.into(),
        }
    }

    /// Create a new tunnel operation error
    pub fn tunnel_operation(message: impl Into<String>) -> Self {
        Self::TunnelOperation {
            message: message.into(),
        }
    }

    /// Create a new process execution error
    pub fn process_execution(message: impl Into<String>) -> Self {
        Self::ProcessExecution {
            message: message.into(),
        }
    }

    /// Create a new connection error
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection {
            message: message.into(),
        }
    }

    /// Create a new LocalProxy startup error
    pub fn localproxy_startup(message: impl Into<String>) -> Self {
        Self::LocalProxyStartup {
            message: message.into(),
        }
    }
}

// Convert AWS SDK errors to our custom error type
impl<E> From<SdkError<E>> for TunnelError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: SdkError<E>) -> Self {
        match err {
            SdkError::DispatchFailure(_) => TunnelError::AwsAuth {
                message: "Authentication failed. Please run 'aws sso login' to authenticate.".to_string(),
            },
            _ => TunnelError::AwsSdk(err.to_string()),
        }
    }
}

/// Result type alias for tunnel operations
pub type TunnelResult<T> = Result<T, TunnelError>;

/// UI-specific error types for display purposes
#[derive(Error, Debug, Clone)]
pub enum UiError {
    #[error("Device ID cannot be empty")]
    EmptyDeviceId,

    #[error("Failed to connect: {message}")]
    ConnectionFailed { message: String },

    #[error("Failed to disconnect: {message}")]
    DisconnectionFailed { message: String },

    #[error("Authentication required. Please try again after logging in.")]
    AuthenticationRequired,

    #[error("Unknown error occurred")]
    Unknown,
}

impl From<TunnelError> for UiError {
    fn from(err: TunnelError) -> Self {
        match err {
            TunnelError::AwsAuth { .. } => UiError::AuthenticationRequired,
            TunnelError::InvalidDeviceId { .. } => UiError::EmptyDeviceId,
            TunnelError::Connection { message } => UiError::ConnectionFailed { message },
            _ => UiError::ConnectionFailed {
                message: err.to_string(),
            },
        }
    }
}

impl UiError {
    /// Get a user-friendly message for display in the UI
    pub fn user_message(&self) -> &str {
        match self {
            UiError::EmptyDeviceId => "Please enter a device ID",
            UiError::ConnectionFailed { message } => message,
            UiError::DisconnectionFailed { message } => message,
            UiError::AuthenticationRequired => "Authentication required. Please try connecting again.",
            UiError::Unknown => "An unexpected error occurred",
        }
    }

    /// Check if this error should trigger a retry prompt
    pub fn should_retry(&self) -> bool {
        matches!(self, UiError::AuthenticationRequired)
    }
}
