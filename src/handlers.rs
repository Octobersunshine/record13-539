use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::errors::AppResult;
use crate::models::*;
use crate::services::AuctionService;

pub async fn create_product(
    State(service): State<Arc<AuctionService>>,
    Json(req): Json<CreateProductRequest>,
) -> AppResult<Json<ProductResponse>> {
    let product = service.create_product(&req).await?;
    Ok(Json(product))
}

pub async fn get_product(
    State(service): State<Arc<AuctionService>>,
    Path(product_id): Path<Uuid>,
) -> AppResult<Json<ProductResponse>> {
    let product = service.get_product(product_id).await?;
    Ok(Json(product))
}

pub async fn get_products_by_room(
    State(service): State<Arc<AuctionService>>,
    Path(room_id): Path<String>,
) -> AppResult<Json<Vec<ProductResponse>>> {
    let products = service.get_products_by_room(&room_id).await?;
    Ok(Json(products))
}

pub async fn place_bid(
    State(service): State<Arc<AuctionService>>,
    Json(req): Json<PlaceBidRequest>,
) -> AppResult<Json<BidResponse>> {
    let bid = service.place_bid(&req).await?;
    Ok(Json(bid))
}

pub async fn confirm_purchase(
    State(service): State<Arc<AuctionService>>,
    Path(bid_id): Path<Uuid>,
    Json(body): Json<ConfirmPurchaseRequest>,
) -> AppResult<Json<SuccessResponse>> {
    service.confirm_purchase(bid_id, &body.user_id).await?;
    Ok(Json(SuccessResponse {
        success: true,
        message: "Purchase confirmed successfully".to_string(),
    }))
}

pub async fn cancel_bid(
    State(service): State<Arc<AuctionService>>,
    Path(bid_id): Path<Uuid>,
    Json(body): Json<CancelBidRequest>,
) -> AppResult<Json<SuccessResponse>> {
    service.cancel_bid(bid_id, &body.user_id).await?;
    Ok(Json(SuccessResponse {
        success: true,
        message: "Bid cancelled successfully".to_string(),
    }))
}

pub async fn get_bid(
    State(service): State<Arc<AuctionService>>,
    Path(bid_id): Path<Uuid>,
) -> AppResult<Json<BidResponse>> {
    let bid = service.get_bid(bid_id).await?;
    Ok(Json(bid))
}

pub async fn get_product_locks(
    State(service): State<Arc<AuctionService>>,
    Path(product_id): Path<Uuid>,
) -> AppResult<Json<Vec<StockLock>>> {
    let locks = service.get_product_locks(product_id).await?;
    Ok(Json(locks))
}

pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmPurchaseRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelBidRequest {
    pub user_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
}
