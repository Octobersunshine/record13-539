use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::db;
use crate::errors::{AppError, AppResult};
use crate::models::*;

pub struct AuctionService {
    pool: SqlitePool,
}

impl AuctionService {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_product(&self, req: &CreateProductRequest) -> AppResult<ProductResponse> {
        if req.total_stock <= 0 {
            return Err(AppError::InvalidRequest(
                "Total stock must be greater than 0".to_string(),
            ));
        }
        if req.start_price <= 0.0 {
            return Err(AppError::InvalidRequest(
                "Start price must be greater than 0".to_string(),
            ));
        }
        if req.min_increment <= 0.0 {
            return Err(AppError::InvalidRequest(
                "Min increment must be greater than 0".to_string(),
            ));
        }

        let product = db::create_product(&self.pool, req).await?;
        Ok(ProductResponse {
            id: product.id,
            name: product.name,
            description: product.description,
            total_stock: product.total_stock,
            available_stock: product.available_stock,
            locked_stock: product.locked_stock,
            start_price: product.start_price,
            current_price: product.current_price,
            min_increment: product.min_increment,
            room_id: product.room_id,
            created_at: product.created_at,
        })
    }

    pub async fn get_product(&self, product_id: Uuid) -> AppResult<ProductResponse> {
        let product = db::get_product(&self.pool, product_id)
            .await?
            .ok_or(AppError::ProductNotFound)?;

        Ok(ProductResponse {
            id: product.id,
            name: product.name,
            description: product.description,
            total_stock: product.total_stock,
            available_stock: product.available_stock,
            locked_stock: product.locked_stock,
            start_price: product.start_price,
            current_price: product.current_price,
            min_increment: product.min_increment,
            room_id: product.room_id,
            created_at: product.created_at,
        })
    }

    pub async fn get_products_by_room(&self, room_id: &str) -> AppResult<Vec<ProductResponse>> {
        let products = db::get_products_by_room(&self.pool, room_id).await?;
        Ok(products
            .into_iter()
            .map(|p| ProductResponse {
                id: p.id,
                name: p.name,
                description: p.description,
                total_stock: p.total_stock,
                available_stock: p.available_stock,
                locked_stock: p.locked_stock,
                start_price: p.start_price,
                current_price: p.current_price,
                min_increment: p.min_increment,
                room_id: p.room_id,
                created_at: p.created_at,
            })
            .collect())
    }

    pub async fn place_bid(&self, req: &PlaceBidRequest) -> AppResult<BidResponse> {
        if req.quantity <= 0 {
            return Err(AppError::InvalidRequest(
                "Quantity must be greater than 0".to_string(),
            ));
        }
        if req.bid_price <= 0.0 {
            return Err(AppError::InvalidRequest(
                "Bid price must be greater than 0".to_string(),
            ));
        }

        if let Some(ref idempotency_key) = req.idempotency_key {
            if !idempotency_key.is_empty() {
                if let Some(existing_bid_id) =
                    db::get_idempotency_result(&self.pool, idempotency_key).await?
                {
                    tracing::info!(
                        "Idempotency hit: key={}, bid_id={}",
                        idempotency_key,
                        existing_bid_id
                    );
                    return self.get_bid(existing_bid_id).await;
                }
            }
        }

        let product = db::get_product(&self.pool, req.product_id)
            .await?
            .ok_or(AppError::ProductNotFound)?;

        if req.bid_price < product.current_price {
            return Err(AppError::BidPriceTooLow);
        }

        let min_required_price = product.current_price + product.min_increment;
        if req.bid_price < min_required_price {
            return Err(AppError::InvalidBidIncrement);
        }

        let mut tx = self.pool.begin().await?;

        let released_count =
            db::release_user_active_locks(&mut *tx, req.product_id, &req.user_id).await?;

        if released_count > 0 {
            tracing::info!(
                "Released {} previous locks for user {} on product {}",
                released_count,
                req.user_id,
                req.product_id
            );
        }

        let product_after_release = db::get_product(&mut *tx, req.product_id)
            .await?
            .ok_or(AppError::ProductNotFound)?;

        if product_after_release.available_stock < req.quantity {
            tx.rollback().await?;
            return Err(AppError::InsufficientStock);
        }

        let bid = db::create_bid(
            &mut *tx,
            req.product_id,
            &req.user_id,
            req.bid_price,
            req.quantity,
        )
        .await?;

        let lock =
            db::lock_stock(&mut *tx, req.product_id, bid.id, &req.user_id, req.quantity).await?;

        db::update_product_price(&mut *tx, req.product_id, req.bid_price).await?;
        db::update_bid_status(&mut *tx, bid.id, BidStatus::Confirmed).await?;

        if let Some(ref idempotency_key) = req.idempotency_key {
            if !idempotency_key.is_empty() {
                db::set_idempotency_result(&mut *tx, idempotency_key, bid.id).await?;
            }
        }

        tx.commit().await?;

        tracing::info!(
            "Bid placed: product={}, user={}, price={}, quantity={}, lock_expires={}, released_previous={}",
            req.product_id,
            req.user_id,
            req.bid_price,
            req.quantity,
            lock.expires_at,
            released_count
        );

        Ok(BidResponse {
            id: bid.id,
            product_id: bid.product_id,
            user_id: bid.user_id,
            bid_price: bid.bid_price,
            quantity: bid.quantity,
            status: BidStatus::Confirmed,
            lock_expires_at: lock.expires_at,
            created_at: bid.created_at,
        })
    }

    pub async fn confirm_purchase(&self, bid_id: Uuid, user_id: &str) -> AppResult<()> {
        let bid = db::get_bid(&self.pool, bid_id)
            .await?
            .ok_or(AppError::BidNotFound)?;

        if bid.user_id != user_id {
            return Err(AppError::InvalidRequest(
                "Not authorized to confirm this bid".to_string(),
            ));
        }

        if bid.status != BidStatus::Confirmed {
            return Err(AppError::InvalidRequest(format!(
                "Bid status is {:?}, cannot confirm",
                bid.status
            )));
        }

        let lock = db::get_active_lock_by_bid(&self.pool, bid_id)
            .await?
            .ok_or(AppError::LockExpired)?;

        if lock.expires_at < Utc::now() {
            db::expire_old_locks(&self.pool).await?;
            return Err(AppError::LockExpired);
        }

        db::confirm_lock(&self.pool, lock.id).await?;
        db::update_bid_status(&self.pool, bid_id, BidStatus::Confirmed).await?;

        tracing::info!("Purchase confirmed: bid={}, user={}", bid_id, user_id);

        Ok(())
    }

    pub async fn cancel_bid(&self, bid_id: Uuid, user_id: &str) -> AppResult<()> {
        let bid = db::get_bid(&self.pool, bid_id)
            .await?
            .ok_or(AppError::BidNotFound)?;

        if bid.user_id != user_id {
            return Err(AppError::InvalidRequest(
                "Not authorized to cancel this bid".to_string(),
            ));
        }

        if bid.status != BidStatus::Confirmed {
            return Err(AppError::InvalidRequest(format!(
                "Bid status is {:?}, cannot cancel",
                bid.status
            )));
        }

        let lock = db::get_active_lock_by_bid(&self.pool, bid_id)
            .await?
            .ok_or_else(|| AppError::InvalidRequest("No active lock for this bid".to_string()))?;

        db::release_lock(&self.pool, lock.id).await?;
        db::update_bid_status(&self.pool, bid_id, BidStatus::Cancelled).await?;

        tracing::info!("Bid cancelled: bid={}, user={}", bid_id, user_id);

        Ok(())
    }

    pub async fn get_bid(&self, bid_id: Uuid) -> AppResult<BidResponse> {
        let bid = db::get_bid(&self.pool, bid_id)
            .await?
            .ok_or(AppError::BidNotFound)?;

        let lock = db::get_active_lock_by_bid(&self.pool, bid_id).await?;
        let lock_expires_at = lock.map(|l| l.expires_at).unwrap_or_else(|| bid.created_at);

        Ok(BidResponse {
            id: bid.id,
            product_id: bid.product_id,
            user_id: bid.user_id,
            bid_price: bid.bid_price,
            quantity: bid.quantity,
            status: bid.status,
            lock_expires_at,
            created_at: bid.created_at,
        })
    }

    pub async fn get_product_locks(&self, product_id: Uuid) -> AppResult<Vec<StockLock>> {
        let _product = db::get_product(&self.pool, product_id)
            .await?
            .ok_or(AppError::ProductNotFound)?;

        let locks = db::get_product_locks(&self.pool, product_id).await?;
        Ok(locks)
    }

    pub async fn cleanup_expired_locks(&self) -> AppResult<i64> {
        db::expire_old_locks(&self.pool).await
    }

    pub async fn cleanup_expired_idempotency_keys(&self) -> AppResult<i64> {
        db::cleanup_expired_idempotency_keys(&self.pool).await
    }
}
