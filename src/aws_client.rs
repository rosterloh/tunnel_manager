use async_trait::async_trait;
use aws_sdk_iotsecuretunneling::{
    operation::{
        list_tunnels::{ListTunnelsError, ListTunnelsOutput},
        open_tunnel::{OpenTunnelError, OpenTunnelOutput},
        rotate_tunnel_access_token::{RotateTunnelAccessTokenError, RotateTunnelAccessTokenOutput},
        close_tunnel::{CloseTunnelError, CloseTunnelOutput},
    },
    error::SdkError,
    types::{ClientMode, DestinationConfig},
};

/// Trait for AWS IoT Secure Tunneling operations to enable mocking
#[async_trait]
pub trait TunnelClient: Send + Sync {
    async fn list_tunnels_for_thing(&self, thing_name: &str) -> Result<ListTunnelsOutput, SdkError<ListTunnelsError>>;
    
    async fn open_tunnel_with_config(&self, dest_config: DestinationConfig) -> Result<OpenTunnelOutput, SdkError<OpenTunnelError>>;
    
    async fn rotate_tunnel_tokens(
        &self,
        tunnel_id: &str,
        client_mode: ClientMode,
        dest_config: DestinationConfig,
    ) -> Result<RotateTunnelAccessTokenOutput, SdkError<RotateTunnelAccessTokenError>>;
    
    async fn close_tunnel_by_id(&self, tunnel_id: &str) -> Result<CloseTunnelOutput, SdkError<CloseTunnelError>>;
}

/// Real AWS client implementation
pub struct AwsTunnelClient {
    client: aws_sdk_iotsecuretunneling::Client,
}

impl AwsTunnelClient {
    pub fn new(client: aws_sdk_iotsecuretunneling::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TunnelClient for AwsTunnelClient {
    async fn list_tunnels_for_thing(&self, thing_name: &str) -> Result<ListTunnelsOutput, SdkError<ListTunnelsError>> {
        self.client
            .list_tunnels()
            .thing_name(thing_name)
            .send()
            .await
    }
    
    async fn open_tunnel_with_config(&self, dest_config: DestinationConfig) -> Result<OpenTunnelOutput, SdkError<OpenTunnelError>> {
        self.client
            .open_tunnel()
            .destination_config(dest_config)
            .send()
            .await
    }
    
    async fn rotate_tunnel_tokens(
        &self,
        tunnel_id: &str,
        client_mode: ClientMode,
        dest_config: DestinationConfig,
    ) -> Result<RotateTunnelAccessTokenOutput, SdkError<RotateTunnelAccessTokenError>> {
        self.client
            .rotate_tunnel_access_token()
            .tunnel_id(tunnel_id)
            .client_mode(client_mode)
            .destination_config(dest_config)
            .send()
            .await
    }
    
    async fn close_tunnel_by_id(&self, tunnel_id: &str) -> Result<CloseTunnelOutput, SdkError<CloseTunnelError>> {
        self.client
            .close_tunnel()
            .tunnel_id(tunnel_id)
            .send()
            .await
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use mockall::mock;
    
    mock! {
        pub TunnelClient {}
        
        #[async_trait]
        impl TunnelClient for TunnelClient {
            async fn list_tunnels_for_thing(&self, thing_name: &str) -> Result<ListTunnelsOutput, SdkError<ListTunnelsError>>;
            async fn open_tunnel_with_config(&self, dest_config: DestinationConfig) -> Result<OpenTunnelOutput, SdkError<OpenTunnelError>>;
            async fn rotate_tunnel_tokens(
                &self,
                tunnel_id: &str,
                client_mode: ClientMode,
                dest_config: DestinationConfig,
            ) -> Result<RotateTunnelAccessTokenOutput, SdkError<RotateTunnelAccessTokenError>>;
            async fn close_tunnel_by_id(&self, tunnel_id: &str) -> Result<CloseTunnelOutput, SdkError<CloseTunnelError>>;
        }
    }
}
