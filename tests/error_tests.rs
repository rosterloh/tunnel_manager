use tunnel_manager::error::{TunnelError, UiError, TunnelResult};
use std::io;

#[test]
fn test_tunnel_error_display() {
    let error = TunnelError::InvalidDeviceId {
        device_id: "test-device".to_string(),
    };
    assert_eq!(error.to_string(), "Invalid device ID: test-device");
}

#[test]
fn test_tunnel_error_creation_helpers() {
    let error = TunnelError::aws_config("Configuration failed");
    assert!(matches!(error, TunnelError::AwsConfig { .. }));
    
    let error = TunnelError::connection("Connection timeout");
    assert!(matches!(error, TunnelError::Connection { .. }));
}

#[test]
fn test_io_error_conversion() {
    let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
    let tunnel_error: TunnelError = io_error.into();
    assert!(matches!(tunnel_error, TunnelError::Io(_)));
}

#[test]
fn test_tunnel_error_to_ui_error_conversion() {
    let tunnel_error = TunnelError::AwsAuth {
        message: "Auth failed".to_string(),
    };
    let ui_error: UiError = tunnel_error.into();
    assert!(matches!(ui_error, UiError::AuthenticationRequired));
    
    let tunnel_error = TunnelError::InvalidDeviceId {
        device_id: "test".to_string(),
    };
    let ui_error: UiError = tunnel_error.into();
    assert!(matches!(ui_error, UiError::EmptyDeviceId));
}

#[test]
fn test_ui_error_user_messages() {
    let error = UiError::EmptyDeviceId;
    assert_eq!(error.user_message(), "Please enter a device ID");
    
    let error = UiError::AuthenticationRequired;
    assert_eq!(error.user_message(), "Authentication required. Please try connecting again.");
    
    let error = UiError::ConnectionFailed {
        message: "Network error".to_string(),
    };
    assert_eq!(error.user_message(), "Network error");
}

#[test]
fn test_ui_error_should_retry() {
    assert!(UiError::AuthenticationRequired.should_retry());
    assert!(!UiError::EmptyDeviceId.should_retry());
    assert!(!UiError::Unknown.should_retry());
}

#[test]
fn test_tunnel_result_type() {
    fn test_function() -> TunnelResult<String> {
        Ok("success".to_string())
    }
    
    let result = test_function();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
}

#[test]
fn test_token_rotation_error() {
    let error = TunnelError::TokenRotation {
        tunnel_id: "tunnel-123".to_string(),
        message: "Token expired".to_string(),
    };
    assert_eq!(
        error.to_string(),
        "Token rotation failed for tunnel tunnel-123: Token expired"
    );
}

#[test]
fn test_tunnel_not_found_error() {
    let error = TunnelError::TunnelNotFound {
        device_id: "device-456".to_string(),
    };
    assert_eq!(error.to_string(), "Tunnel not found for device: device-456");
}
