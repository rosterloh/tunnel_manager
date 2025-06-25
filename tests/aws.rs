use tunnel_manager::aws::get_client;

#[tokio::test]
async fn list_all_tunnels() {
    let client = get_client()
        .await
        .expect("Failed to create AWS IoT Secure Tunneling client");

    let device_id = "G111070";
    match client.list_tunnels().thing_name(device_id).send().await {
        Ok(response) => {
            if let Some(tunnel_summaries) = response.tunnel_summaries {
                if tunnel_summaries.is_empty() {
                    println!("No tunnels found for device ID: {}", device_id);
                } else {
                    for tunnel in tunnel_summaries {
                        println!(
                            "Tunnel ID: {}, Status: {:?}",
                            tunnel.tunnel_id.unwrap_or_default(),
                            tunnel.status
                        );
                    }
                }
            } else {
                println!("No tunnels found for device ID: {}", device_id);
            }
        }
        Err(e) => {
            eprintln!("Error listing tunnels: {}", e);
        }
    }
}
