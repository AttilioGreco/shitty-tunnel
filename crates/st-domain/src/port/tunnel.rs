use async_trait::async_trait;

use crate::error::DomainError;
use crate::model::request::{ProxiedRequest, ProxiedResponse};

#[async_trait]
pub trait TunnelTransport: Send + Sync {
    async fn send_request(&self, req: ProxiedRequest) -> Result<ProxiedResponse, DomainError>;
}
