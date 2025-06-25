use tunnel_manager::error::{TunnelError, UiError};
use std::time::{Duration, Instant};

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn test_error_creation_performance() {
        let start = Instant::now();
        
        // Create 1000 errors to test performance
        for i in 0..1000 {
            let _error = TunnelError::InvalidDeviceId {
                device_id: format!("device-{}", i),
            };
        }
        
        let duration = start.elapsed();
        // Should complete in less than 1ms for 1000 error creations
        assert!(duration < Duration::from_millis(1));
    }

    #[test]
    fn test_error_conversion_performance() {
        let start = Instant::now();
        
        // Test error conversion performance
        for i in 0..1000 {
            let tunnel_error = TunnelError::AwsAuth {
                message: format!("Auth failed for attempt {}", i),
            };
            let _ui_error: UiError = tunnel_error.into();
        }
        
        let duration = start.elapsed();
        // Should complete in less than 2ms for 1000 conversions
        assert!(duration < Duration::from_millis(2));
    }

    #[test]
    fn test_error_display_performance() {
        let start = Instant::now();
        
        // Test error display formatting performance
        for i in 0..1000 {
            let error = TunnelError::TunnelNotFound {
                device_id: format!("device-{}", i),
            };
            let _display_string = error.to_string();
        }
        
        let duration = start.elapsed();
        // Should complete in less than 5ms for 1000 display operations
        assert!(duration < Duration::from_millis(5));
    }

    #[test]
    fn test_ui_error_message_retrieval_performance() {
        let errors = vec![
            UiError::EmptyDeviceId,
            UiError::AuthenticationRequired,
            UiError::ConnectionFailed {
                message: "Network timeout".to_string(),
            },
            UiError::DisconnectionFailed {
                message: "Process not found".to_string(),
            },
            UiError::Unknown,
        ];

        let start = Instant::now();
        
        // Test UI error message retrieval performance
        for _ in 0..1000 {
            for error in &errors {
                let _message = error.user_message();
                let _should_retry = error.should_retry();
            }
        }
        
        let duration = start.elapsed();
        // Should complete in less than 2ms for 5000 operations (1000 iterations * 5 errors)
        assert!(duration < Duration::from_millis(2));
    }

    #[tokio::test]
    async fn test_async_error_handling_performance() {
        async fn mock_async_operation(should_succeed: bool) -> Result<String, TunnelError> {
            if should_succeed {
                Ok("Success".to_string())
            } else {
                Err(TunnelError::connection("Mock connection failed"))
            }
        }

        let start = Instant::now();
        
        // Test async error handling performance
        for i in 0..1000 {
            let result = mock_async_operation(i % 2 == 0).await;
            match result {
                Ok(_) => {
                    // Handle success case
                }
                Err(e) => {
                    let _ui_error: UiError = e.into();
                }
            }
        }
        
        let duration = start.elapsed();
        // Should complete in less than 10ms for 1000 async operations
        assert!(duration < Duration::from_millis(10));
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[test]
    fn test_large_error_message_handling() {
        // Test with very large error messages
        let large_message = "A".repeat(10000);
        let error = TunnelError::connection(large_message.clone());
        
        assert_eq!(error.to_string(), format!("Connection failed: {}", large_message));
    }

    #[test]
    fn test_nested_error_conversions() {
        // Test multiple levels of error conversions
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let tunnel_error: TunnelError = io_error.into();
        let ui_error: UiError = tunnel_error.into();
        
        assert!(matches!(ui_error, UiError::ConnectionFailed { .. }));
    }

    #[test]
    fn test_concurrent_error_creation() {
        use std::thread;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];

        // Spawn 10 threads creating errors concurrently
        for thread_id in 0..10 {
            let counter_clone = counter.clone();
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let _error = TunnelError::InvalidDeviceId {
                        device_id: format!("thread-{}-device-{}", thread_id, i),
                    };
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all errors were created
        assert_eq!(counter.load(Ordering::SeqCst), 1000);
    }
}
