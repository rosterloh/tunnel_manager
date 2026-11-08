// Mock return closures have to match the AWS SDK's large SdkError
#![allow(clippy::result_large_err)]

use aws_sdk_iotsecuretunneling::error::SdkError;
use aws_sdk_iotsecuretunneling::operation::close_tunnel::CloseTunnelOutput;
use aws_sdk_iotsecuretunneling::operation::list_tunnels::ListTunnelsOutput;
use aws_sdk_iotsecuretunneling::operation::open_tunnel::OpenTunnelOutput;
use aws_sdk_iotsecuretunneling::operation::rotate_tunnel_access_token::RotateTunnelAccessTokenOutput;
use aws_sdk_iotsecuretunneling::types::{
    ClientMode, DestinationConfig, TunnelStatus, TunnelSummary,
};
use aws_smithy_runtime_api::client::result::ConnectorError;
use mockall::predicate::{always, eq};
use tokio::process::Command;
use tunnel_manager::aws::{TunnelConnection, close_connection, open_tunnel_for_device};
use tunnel_manager::aws_client::test_utils::MockTunnelClient;
use tunnel_manager::error::TunnelError;

const DEVICE: &str = "G111070";

fn listing(tunnels: &[(&str, TunnelStatus)]) -> ListTunnelsOutput {
    let summaries = tunnels
        .iter()
        .map(|(id, status)| {
            TunnelSummary::builder()
                .tunnel_id(*id)
                .status(status.clone())
                .build()
        })
        .collect();
    ListTunnelsOutput::builder()
        .set_tunnel_summaries(Some(summaries))
        .build()
}

fn expect_listing(mock: &mut MockTunnelClient, tunnels: &'static [(&str, TunnelStatus)]) {
    mock.expect_list_tunnels_for_thing()
        .with(eq(DEVICE))
        .times(1)
        .returning(move |_| Ok(listing(tunnels)));
}

fn targets_device(config: &DestinationConfig) -> bool {
    config.thing_name() == Some(DEVICE) && config.services() == ["GORT", "SSH"]
}

fn expect_new_tunnel(mock: &mut MockTunnelClient) {
    mock.expect_open_tunnel_with_config()
        .withf(targets_device)
        .times(1)
        .returning(|_| {
            Ok(OpenTunnelOutput::builder()
                .tunnel_id("new-tunnel")
                .source_access_token("new-source-token")
                .destination_access_token("new-dest-token")
                .build())
        });
}

fn expect_rotation(mock: &mut MockTunnelClient, tunnel_id: &'static str) {
    mock.expect_rotate_tunnel_tokens()
        .withf(move |id, mode, config| {
            id == tunnel_id && *mode == ClientMode::All && targets_device(config)
        })
        .times(1)
        .returning(|_, _, _| {
            Ok(RotateTunnelAccessTokenOutput::builder()
                .source_access_token("rotated-source-token")
                .destination_access_token("rotated-dest-token")
                .build())
        });
}

fn expect_close(mock: &mut MockTunnelClient, tunnel_id: &'static str) {
    mock.expect_close_tunnel_by_id()
        .with(eq(tunnel_id))
        .times(1)
        .returning(|_| Ok(CloseTunnelOutput::builder().build()));
}

fn tunnel(id: &str, token: &str) -> (String, String) {
    (id.to_string(), token.to_string())
}

#[tokio::test]
async fn opens_a_new_tunnel_when_device_has_none() {
    let mut mock = MockTunnelClient::new();
    expect_listing(&mut mock, &[]);
    expect_new_tunnel(&mut mock);

    let result = open_tunnel_for_device(&mock, DEVICE).await.unwrap();

    assert_eq!(result, tunnel("new-tunnel", "new-source-token"));
}

#[tokio::test]
async fn reuses_an_open_tunnel_by_rotating_tokens() {
    let mut mock = MockTunnelClient::new();
    expect_listing(&mut mock, &[("open-tunnel", TunnelStatus::Open)]);
    expect_rotation(&mut mock, "open-tunnel");

    let result = open_tunnel_for_device(&mock, DEVICE).await.unwrap();

    assert_eq!(result, tunnel("open-tunnel", "rotated-source-token"));
}

#[tokio::test]
async fn closes_stale_tunnels_listed_before_the_open_one() {
    let mut mock = MockTunnelClient::new();
    expect_listing(
        &mut mock,
        &[
            ("stale-tunnel", TunnelStatus::Closed),
            ("open-tunnel", TunnelStatus::Open),
        ],
    );
    expect_close(&mut mock, "stale-tunnel");
    expect_rotation(&mut mock, "open-tunnel");

    let result = open_tunnel_for_device(&mock, DEVICE).await.unwrap();

    assert_eq!(result, tunnel("open-tunnel", "rotated-source-token"));
}

#[tokio::test]
async fn opens_a_new_tunnel_after_closing_stale_ones() {
    let mut mock = MockTunnelClient::new();
    expect_listing(&mut mock, &[("stale-tunnel", TunnelStatus::Closed)]);
    expect_close(&mut mock, "stale-tunnel");
    expect_new_tunnel(&mut mock);

    let result = open_tunnel_for_device(&mock, DEVICE).await.unwrap();

    assert_eq!(result, tunnel("new-tunnel", "new-source-token"));
}

#[tokio::test]
async fn missing_source_token_is_an_error_not_a_panic() {
    let mut mock = MockTunnelClient::new();
    expect_listing(&mut mock, &[]);
    mock.expect_open_tunnel_with_config()
        .with(always())
        .times(1)
        .returning(|_| Ok(OpenTunnelOutput::builder().tunnel_id("new-tunnel").build()));

    let error = open_tunnel_for_device(&mock, DEVICE).await.unwrap_err();

    assert!(matches!(error, TunnelError::TunnelOperation { .. }));
}

async fn listing_failure(connector_error: ConnectorError) -> TunnelError {
    let mut mock = MockTunnelClient::new();
    mock.expect_list_tunnels_for_thing()
        .times(1)
        .return_once(move |_| Err(SdkError::dispatch_failure(connector_error)));

    open_tunnel_for_device(&mock, DEVICE).await.unwrap_err()
}

#[tokio::test]
async fn untyped_dispatch_failure_means_login_is_needed() {
    let error = listing_failure(ConnectorError::other("no credentials".into(), None)).await;

    assert!(matches!(error, TunnelError::AwsAuth { .. }));
}

#[tokio::test]
async fn network_failures_do_not_trigger_login() {
    let io = listing_failure(ConnectorError::io("connection refused".into())).await;
    let timeout = listing_failure(ConnectorError::timeout("timed out".into())).await;

    assert!(matches!(io, TunnelError::AwsSdk(_)));
    assert!(matches!(timeout, TunnelError::AwsSdk(_)));
}

fn running_connection() -> (TunnelConnection, String) {
    let proxy = Command::new("sleep")
        .arg("30")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let proc_path = format!("/proc/{}", proxy.id().unwrap());
    let connection = TunnelConnection {
        tunnel_id: "open-tunnel".to_string(),
        proxy,
    };
    (connection, proc_path)
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn disconnect_stops_proxy_and_closes_tunnel() {
    let mut mock = MockTunnelClient::new();
    expect_close(&mut mock, "open-tunnel");
    let (connection, proc_path) = running_connection();

    close_connection(&mock, connection).await.unwrap();

    assert!(!std::path::Path::new(&proc_path).exists());
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn proxy_is_stopped_even_when_closing_the_tunnel_fails() {
    let mut mock = MockTunnelClient::new();
    mock.expect_close_tunnel_by_id()
        .with(eq("open-tunnel"))
        .times(1)
        .return_once(|_| {
            Err(SdkError::dispatch_failure(ConnectorError::io(
                "down".into(),
            )))
        });
    let (connection, proc_path) = running_connection();

    let error = close_connection(&mock, connection).await.unwrap_err();

    assert!(matches!(error, TunnelError::AwsSdk(_)));
    assert!(!std::path::Path::new(&proc_path).exists());
}
