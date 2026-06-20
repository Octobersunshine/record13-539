use std::sync::Arc;
use std::time::Duration;
use tokio::time;

use crate::services::AuctionService;

pub async fn start_expired_locks_cleanup(service: Arc<AuctionService>, interval_seconds: u64) {
    let mut interval = time::interval(Duration::from_secs(interval_seconds));

    loop {
        interval.tick().await;

        match service.cleanup_expired_locks().await {
            Ok(count) if count > 0 => {
                tracing::info!("Cleaned up {} expired locks", count);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to clean up expired locks: {}", e);
            }
        }

        match service.cleanup_expired_idempotency_keys().await {
            Ok(count) if count > 0 => {
                tracing::info!("Cleaned up {} expired idempotency keys", count);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to clean up idempotency keys: {}", e);
            }
        }

        match service.cleanup_ended_auctions().await {
            Ok(count) if count > 0 => {
                tracing::info!("Released {} locks from ended auctions", count);
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Failed to clean up ended auctions: {}", e);
            }
        }
    }
}
