use tokio::process::{Child, Command};

use aws_config::{BehaviorVersion, Region};
use aws_sdk_iotsecuretunneling::{
    Client,
    error::SdkError,
    types::{ClientMode, DestinationConfig, TunnelStatus},
};

use crate::error::{TunnelError, TunnelResult};

const PROFILE: &str = "iotmgmt_prod";
const REGION: &str = "eu-west-1";

async fn open_tunnel(client: &Client, device_id: &str) -> TunnelResult<(String, String, String)> {
    let dest = DestinationConfig::builder()
        .thing_name(device_id)
        .services("GORT")
        .services("SSH")
        .build()
        .expect("Failed to build DestinationConfig for tunnel");

    let tokens = client
        .open_tunnel()
        .destination_config(dest)
        .send()
        .await
        .map_err(|err| TunnelError::tunnel_operation(format!("Failed to open tunnel: {}", err)))?;

    let tunnel_id = tokens.tunnel_id().unwrap().to_string();
    let src_token = tokens.source_access_token().unwrap().to_string();
    let dst_token = tokens.destination_access_token().unwrap().to_string();

    Ok((tunnel_id, src_token, dst_token))
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

async fn start_localproxy_for_source(region: &str, src_token: &str) -> Result<Child, String> {
    let output = Command::new("localproxy")
        .current_dir("assets")
        .args(["-r", region])
        .args(["-s", "SSH=2222,GORT=5555"])
        .args(["-b", "0.0.0.0"])
        // .args(["-t", &src_token])
        .env("AWSIOT_TUNNEL_ACCESS_TOKEN", src_token)
        .spawn()
        .expect("Failed to execute localproxy command");

    Ok(output)
}

async fn rotate_access_tokens(
    client: &Client,
    device_id: &str,
    tunnel_id: &str,
) -> Result<(String, String), String> {
    let dest = DestinationConfig::builder()
        .thing_name(device_id)
        .services("GORT")
        .services("SSH")
        .build()
        .expect("Failed to build DestinationConfig for tunnel");

    let response = client
        .rotate_tunnel_access_token()
        .tunnel_id(tunnel_id)
        .client_mode(ClientMode::All)
        .destination_config(dest)
        .send()
        .await
        .map_err(|e| {
            format!(
                "Failed to rotate access tokens for tunnel {}: {}",
                tunnel_id, e
            )
        })?;

    let src_token = response.source_access_token().unwrap().to_string();
    let dst_token = response.destination_access_token().unwrap().to_string();

    Ok((src_token, dst_token))
}

async fn open_tunnel_for_device(
    client: &Client,
    device_id: &str,
) -> Result<(String, String), String> {
    match client.list_tunnels().thing_name(device_id).send().await {
        Ok(response) => {
            if let Some(tunnel_summaries) = response.tunnel_summaries {
                if tunnel_summaries.is_empty() {
                    println!("No tunnels found for device ID: {}", device_id)
                }
                // Return first valid tunnel ID
                for tunnel in &tunnel_summaries {
                    if *tunnel.status().unwrap() == TunnelStatus::Open {
                        if tunnel.tunnel_id.is_some() {
                            let tunnel_id = tunnel.tunnel_id.clone().unwrap();
                            println!(
                                "Not Opening a new tunnel. There is a tunnel {} for {} with status {}",
                                tunnel_id,
                                device_id,
                                tunnel.status().unwrap()
                            );
                            let (src_token, _) =
                                rotate_access_tokens(client, device_id, &tunnel_id)
                                    .await
                                    .map_err(|_| "Failed to rotate access tokens".to_string())?;

                            return Ok((tunnel_id, src_token));
                        }
                    } else {
                        println!("Deleting tunnel: {:?}", tunnel);
                        client
                            .close_tunnel()
                            .tunnel_id(tunnel.tunnel_id.clone().unwrap())
                            .send()
                            .await
                            .map_err(|e| format!("Failed to close tunnel: {}", e))?;

                        continue;
                    }
                }
            } else {
                println!("No tunnels found for device ID: {}", device_id);
            }

            let (tunnel_id, src_token, _) = open_tunnel(client, device_id)
                .await
                .map_err(|e| format!("Failed to open tunnel: {}", e))?;

            Ok((tunnel_id, src_token))
        }
        Err(err) => {
            if let SdkError::DispatchFailure(_) = err {
                match aws_sso_login().await {
                    Ok(_) => {
                        return Err(String::from("Login successful, please try again."));
                        // Retry the operation after successful login
                        // return get_open_tunnels_for_device(client, device_id).await;
                    }
                    Err(e) => return Err(e.to_string()),
                }
            }
            Err(format!("Failed to list tunnels: {}", err))
        }
    }
}

pub async fn connect_to_tunnel(device_id: &str) -> Result<Child, String> {
    let client = get_client().await?;
    let region = client
        .config()
        .region()
        .unwrap_or(&Region::from_static(REGION))
        .to_string();

    match open_tunnel_for_device(&client, device_id).await {
        Ok((tunnel_id, src_token)) => {
            println!("Tunnel {} open for device {}", tunnel_id, device_id);
            let child = start_localproxy_for_source(&region, &src_token)
                .await
                .map_err(|e| format!("Failed to start localproxy: {}", e))?;

            Ok(child)
        }
        Err(e) => Err(format!("Error retrieving tunnels: {}", e)),
    }
}

pub async fn get_client() -> Result<Client, String> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .profile_name(PROFILE)
        .region(Region::new(REGION))
        .load()
        .await;

    Ok(Client::new(&config))
}
