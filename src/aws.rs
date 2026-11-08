use tokio::process::{Child, Command};

use aws_config::{BehaviorVersion, Region};
use aws_sdk_iotsecuretunneling::{
    Client,
    types::{ClientMode, DestinationConfig, TunnelStatus},
};

use crate::aws_client::{AwsTunnelClient, TunnelClient};
use crate::error::{TunnelError, TunnelResult};

const PROFILE: &str = "iotmgmt_prod";
const REGION: &str = "eu-west-1";

fn destination_config(device_id: &str) -> TunnelResult<DestinationConfig> {
    DestinationConfig::builder()
        .thing_name(device_id)
        .services("GORT")
        .services("SSH")
        .build()
        .map_err(|e| TunnelError::aws_config(e.to_string()))
}

fn required(value: Option<&str>, field: &str) -> TunnelResult<String> {
    value
        .map(str::to_string)
        .ok_or_else(|| TunnelError::tunnel_operation(format!("AWS response is missing {field}")))
}

async fn open_tunnel<C: TunnelClient>(
    client: &C,
    device_id: &str,
) -> TunnelResult<(String, String)> {
    let tokens = client
        .open_tunnel_with_config(destination_config(device_id)?)
        .await?;

    Ok((
        required(tokens.tunnel_id(), "the tunnel ID")?,
        required(tokens.source_access_token(), "the source access token")?,
    ))
}

async fn aws_sso_login() -> TunnelResult<()> {
    let output = Command::new("aws")
        .args(["sso", "login", "--profile", PROFILE])
        .output()
        .await
        .map_err(|e| {
            TunnelError::aws_auth(format!("Failed to execute aws sso login command: {}", e))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(TunnelError::aws_auth(
            "Failed to execute aws sso login command. Please authenticate with aws-cli: aws sso login.",
        ))
    }
}

fn start_localproxy_for_source(src_token: &str) -> TunnelResult<Child> {
    Command::new("localproxy")
        .args(["-r", REGION])
        .args(["-s", "SSH=2222,GORT=5555"])
        .args(["-b", "0.0.0.0"])
        .env("AWSIOT_TUNNEL_ACCESS_TOKEN", src_token)
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| TunnelError::localproxy_startup(e.to_string()))
}

async fn rotate_access_tokens<C: TunnelClient>(
    client: &C,
    device_id: &str,
    tunnel_id: &str,
) -> TunnelResult<String> {
    let response = client
        .rotate_tunnel_tokens(tunnel_id, ClientMode::All, destination_config(device_id)?)
        .await?;

    required(response.source_access_token(), "the source access token")
}

/// Reuses the device's open tunnel if it has one, otherwise opens a new one.
/// Returns the tunnel ID and source access token.
pub async fn open_tunnel_for_device<C: TunnelClient>(
    client: &C,
    device_id: &str,
) -> TunnelResult<(String, String)> {
    let response = client.list_tunnels_for_thing(device_id).await?;

    for tunnel in response.tunnel_summaries() {
        let Some(tunnel_id) = tunnel.tunnel_id() else {
            continue;
        };

        if tunnel.status() == Some(&TunnelStatus::Open) {
            println!("Not opening a new tunnel. Reusing {tunnel_id} for {device_id}");
            let src_token = rotate_access_tokens(client, device_id, tunnel_id).await?;
            return Ok((tunnel_id.to_string(), src_token));
        }

        println!("Deleting tunnel: {tunnel:?}");
        client.close_tunnel_by_id(tunnel_id).await?;
    }

    open_tunnel(client, device_id).await
}

pub async fn connect_to_tunnel(device_id: &str) -> TunnelResult<Child> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .profile_name(PROFILE)
        .region(Region::new(REGION))
        .load()
        .await;
    let client = AwsTunnelClient::new(Client::new(&config));

    match open_tunnel_for_device(&client, device_id).await {
        Ok((tunnel_id, src_token)) => {
            println!("Tunnel {tunnel_id} open for device {device_id}");
            start_localproxy_for_source(&src_token)
        }
        Err(TunnelError::AwsAuth { .. }) => {
            aws_sso_login().await?;
            Err(TunnelError::aws_auth("Login successful, please try again."))
        }
        Err(e) => Err(e),
    }
}
