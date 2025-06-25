use tunnel_manager::aws_client::test_utils::MockTunnelClient;
use tunnel_manager::aws_client::TunnelClient;
use aws_sdk_iotsecuretunneling::types::TunnelStatus;
use aws_sdk_iotsecuretunneling::operation::list_tunnels::ListTunnelsOutput;
use tokio_test::block_on;
use mockall::predicate::*;

#[tokio::test]
async fn test_list_tunnels() {
    let mut mock_client = MockTunnelClient::new();

    mock_client
        .expect_list_tunnels_for_thing()
        .with(eq("test-device"))
        .times(1)
        .returning(|_thing_name| {
            Ok(ListTunnelsOutput::builder()
                .tunnel_summaries(
                    aws_sdk_iotsecuretunneling::types::TunnelSummary::builder()
                        .tunnel_id("tunnel-123")
                        .status(TunnelStatus::Open)
                        .build(),
                )
                .build())
        });

    let result = mock_client.list_tunnels_for_thing("test-device").await;
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.tunnel_summaries.is_some());
    let tunnels = output.tunnel_summaries.unwrap();
    assert_eq!(tunnels.len(), 1);
    assert_eq!(tunnels[0].tunnel_id.as_deref(), Some("tunnel-123"));
    assert_eq!(tunnels[0].status, Some(TunnelStatus::Open));
}

fn main() {
    block_on(async {
        test_list_tunnels().await;
    });
}
