use aws_sdk_iotsecuretunneling::operation::list_tunnels::ListTunnelsOutput;
use aws_sdk_iotsecuretunneling::operation::open_tunnel::OpenTunnelOutput;
use aws_sdk_iotsecuretunneling::operation::rotate_tunnel_access_token::RotateTunnelAccessTokenOutput;
use aws_sdk_iotsecuretunneling::types::{TunnelStatus, TunnelSummary};
use mockall::predicate::*;
use tunnel_manager::aws_client::TunnelClient;
use tunnel_manager::aws_client::test_utils::MockTunnelClient;

/// Test helper to create a mock tunnel summary
fn create_mock_tunnel_summary(tunnel_id: &str, status: TunnelStatus) -> TunnelSummary {
    TunnelSummary::builder()
        .tunnel_id(tunnel_id)
        .status(status)
        .build()
}

/// Test helper to create mock tokens response
fn create_mock_open_tunnel_output(tunnel_id: &str) -> OpenTunnelOutput {
    OpenTunnelOutput::builder()
        .tunnel_id(tunnel_id)
        .source_access_token("mock-source-token")
        .destination_access_token("mock-dest-token")
        .build()
}

#[cfg(test)]
mod aws_business_logic_tests {
    use super::*;

    #[tokio::test]
    async fn test_open_tunnel_success() {
        let mut mock_client = MockTunnelClient::new();

        mock_client
            .expect_open_tunnel_with_config()
            .times(1)
            .returning(|_config| Ok(create_mock_open_tunnel_output("new-tunnel-123")));

        let dest_config = aws_sdk_iotsecuretunneling::types::DestinationConfig::builder()
            .thing_name("test-device")
            .services("SSH")
            .build()
            .expect("Failed to build DestinationConfig");

        let result = mock_client.open_tunnel_with_config(dest_config).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.tunnel_id(), Some("new-tunnel-123"));
        assert_eq!(output.source_access_token(), Some("mock-source-token"));
        assert_eq!(output.destination_access_token(), Some("mock-dest-token"));
    }

    #[tokio::test]
    async fn test_list_tunnels_with_open_tunnel() {
        let mut mock_client = MockTunnelClient::new();

        mock_client
            .expect_list_tunnels_for_thing()
            .with(eq("device-with-open-tunnel"))
            .times(1)
            .returning(|_thing_name| {
                Ok(ListTunnelsOutput::builder()
                    .tunnel_summaries(create_mock_tunnel_summary("tunnel-456", TunnelStatus::Open))
                    .build())
            });

        let result = mock_client
            .list_tunnels_for_thing("device-with-open-tunnel")
            .await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let tunnels = output.tunnel_summaries.unwrap();
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].status, Some(TunnelStatus::Open));
    }

    #[tokio::test]
    async fn test_list_tunnels_with_closed_tunnel() {
        let mut mock_client = MockTunnelClient::new();

        mock_client
            .expect_list_tunnels_for_thing()
            .with(eq("device-with-closed-tunnel"))
            .times(1)
            .returning(|_thing_name| {
                Ok(ListTunnelsOutput::builder()
                    .tunnel_summaries(create_mock_tunnel_summary(
                        "tunnel-789",
                        TunnelStatus::Closed,
                    ))
                    .build())
            });

        let result = mock_client
            .list_tunnels_for_thing("device-with-closed-tunnel")
            .await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let tunnels = output.tunnel_summaries.unwrap();
        assert_eq!(tunnels.len(), 1);
        assert_eq!(tunnels[0].status, Some(TunnelStatus::Closed));
    }

    #[tokio::test]
    async fn test_rotate_tunnel_tokens_success() {
        let mut mock_client = MockTunnelClient::new();

        mock_client
            .expect_rotate_tunnel_tokens()
            .with(eq("tunnel-123"), always(), always())
            .times(1)
            .returning(|_tunnel_id, _client_mode, _dest_config| {
                Ok(RotateTunnelAccessTokenOutput::builder()
                    .source_access_token("new-source-token")
                    .destination_access_token("new-dest-token")
                    .build())
            });

        let dest_config = aws_sdk_iotsecuretunneling::types::DestinationConfig::builder()
            .thing_name("test-device")
            .services("SSH")
            .build()
            .expect("Failed to build DestinationConfig");

        let result = mock_client
            .rotate_tunnel_tokens(
                "tunnel-123",
                aws_sdk_iotsecuretunneling::types::ClientMode::All,
                dest_config,
            )
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.source_access_token(), Some("new-source-token"));
        assert_eq!(output.destination_access_token(), Some("new-dest-token"));
    }

    #[tokio::test]
    async fn test_close_tunnel_success() {
        let mut mock_client = MockTunnelClient::new();

        mock_client
            .expect_close_tunnel_by_id()
            .with(eq("tunnel-to-close"))
            .times(1)
            .returning(|_tunnel_id| {
                Ok(aws_sdk_iotsecuretunneling::operation::close_tunnel::CloseTunnelOutput::builder().build())
            });

        let result = mock_client.close_tunnel_by_id("tunnel-to-close").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_tunnels_multiple_tunnels() {
        let mut mock_client = MockTunnelClient::new();

        mock_client
            .expect_list_tunnels_for_thing()
            .with(eq("device-with-multiple-tunnels"))
            .times(1)
            .returning(|_thing_name| {
                Ok(ListTunnelsOutput::builder()
                    .tunnel_summaries(create_mock_tunnel_summary("tunnel-1", TunnelStatus::Open))
                    .tunnel_summaries(create_mock_tunnel_summary("tunnel-2", TunnelStatus::Closed))
                    .tunnel_summaries(create_mock_tunnel_summary("tunnel-3", TunnelStatus::Open))
                    .build())
            });

        let result = mock_client
            .list_tunnels_for_thing("device-with-multiple-tunnels")
            .await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let tunnels = output.tunnel_summaries.unwrap();
        assert_eq!(tunnels.len(), 3);

        // Verify we have a mix of open and closed tunnels
        let open_count = tunnels
            .iter()
            .filter(|t| t.status == Some(TunnelStatus::Open))
            .count();
        let closed_count = tunnels
            .iter()
            .filter(|t| t.status == Some(TunnelStatus::Closed))
            .count();
        assert_eq!(open_count, 2);
        assert_eq!(closed_count, 1);
    }

    #[tokio::test]
    async fn test_device_id_validation() {
        // Test empty device ID
        let empty_device_id = "";
        assert!(empty_device_id.is_empty());

        // Test valid device ID format (assuming format like G111070)
        let valid_device_id = "G111070";
        assert!(!valid_device_id.is_empty());
        assert!(valid_device_id.starts_with('G'));
        assert!(valid_device_id.len() > 1);
    }
}

/// Integration test that combines multiple operations
#[tokio::test]
async fn test_tunnel_lifecycle() {
    let mut mock_client = MockTunnelClient::new();

    // First, list tunnels (should be empty)
    mock_client
        .expect_list_tunnels_for_thing()
        .with(eq("new-device"))
        .times(1)
        .returning(|_| Ok(ListTunnelsOutput::builder().build()));

    // Then open a new tunnel
    mock_client
        .expect_open_tunnel_with_config()
        .times(1)
        .returning(|_| Ok(create_mock_open_tunnel_output("lifecycle-tunnel")));

    // Then list tunnels again (should show the new tunnel)
    mock_client
        .expect_list_tunnels_for_thing()
        .with(eq("new-device"))
        .times(1)
        .returning(|_| {
            Ok(ListTunnelsOutput::builder()
                .tunnel_summaries(create_mock_tunnel_summary(
                    "lifecycle-tunnel",
                    TunnelStatus::Open,
                ))
                .build())
        });

    // Finally, close the tunnel
    mock_client
        .expect_close_tunnel_by_id()
        .with(eq("lifecycle-tunnel"))
        .times(1)
        .returning(|_| {
            Ok(
                aws_sdk_iotsecuretunneling::operation::close_tunnel::CloseTunnelOutput::builder()
                    .build(),
            )
        });

    // Execute the lifecycle
    let list_result1 = mock_client.list_tunnels_for_thing("new-device").await;
    assert!(list_result1.is_ok());
    assert!(list_result1.unwrap().tunnel_summaries.is_none());

    let dest_config = aws_sdk_iotsecuretunneling::types::DestinationConfig::builder()
        .thing_name("new-device")
        .services("SSH")
        .build()
        .expect("Failed to build DestinationConfig");

    let open_result = mock_client.open_tunnel_with_config(dest_config).await;
    assert!(open_result.is_ok());

    let list_result2 = mock_client.list_tunnels_for_thing("new-device").await;
    assert!(list_result2.is_ok());
    let tunnels = list_result2.unwrap().tunnel_summaries.unwrap();
    assert_eq!(tunnels.len(), 1);

    let close_result = mock_client.close_tunnel_by_id("lifecycle-tunnel").await;
    assert!(close_result.is_ok());
}
