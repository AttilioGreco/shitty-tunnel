use async_trait::async_trait;

use crate::error::DomainError;
use crate::model::request::{ProxiedRequest, ProxiedResponse};

#[async_trait]
pub trait LocalProxy: Send + Sync {
    async fn forward(&self, req: ProxiedRequest) -> Result<ProxiedResponse, DomainError>;
}
