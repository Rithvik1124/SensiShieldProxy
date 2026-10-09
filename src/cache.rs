use crate::scanner::ScanResult;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use tracing::warn;

#[derive(Clone)]
pub struct ScanCache {
    manager: Option<ConnectionManager>,
    ttl_seconds: u64,
}

impl ScanCache {
    pub async fn new(url: &str) -> Self {
        let manager = match redis::Client::open(url) {
            Ok(client) => match client.get_connection_manager().await {
                Ok(manager) => Some(manager),
                Err(e) => {
                    warn!("Redis cache unavailable at startup: {e}; scanning will continue without cache");
                    None
                }
            },
            Err(e) => {
                warn!("Invalid REDIS_URL: {e}; scanning will continue without cache");
                None
            }
        };
        Self { manager, ttl_seconds: 7 * 24 * 60 * 60 }
    }

    pub async fn get(&self, hash: &str) -> Option<ScanResult> {
        let mut conn = self.manager.clone()?;
        let key = format!("scan:{hash}");
        let value: Result<String, _> = conn.get(key).await;
        match value {
            Ok(json) => serde_json::from_str(&json).ok(),
            Err(e) => { warn!("Redis cache read failed: {e}"); None }
        }
    }

    pub async fn put(&self, hash: &str, result: &ScanResult) -> anyhow::Result<()> {
        let Some(mut conn) = self.manager.clone() else { return Ok(()); };
        let key = format!("scan:{hash}");
        let json = serde_json::to_string(result)?;
        let _: () = conn.set_ex(key, json, self.ttl_seconds).await?;
        Ok(())
    }
}
