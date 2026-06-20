use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub total_stock: i64,
    pub available_stock: i64,
    pub locked_stock: i64,
    pub start_price: f64,
    pub current_price: f64,
    pub min_increment: f64,
    pub room_id: String,
    pub end_time: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub description: String,
    pub total_stock: i64,
    pub start_price: f64,
    pub min_increment: f64,
    pub room_id: String,
    #[serde(default)]
    pub auction_duration_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub total_stock: i64,
    pub available_stock: i64,
    pub locked_stock: i64,
    pub start_price: f64,
    pub current_price: f64,
    pub min_increment: f64,
    pub room_id: String,
    pub end_time: Option<DateTime<Utc>>,
    pub auction_status: AuctionStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuctionStatus {
    Upcoming,
    Ongoing,
    Ended,
}

impl Product {
    pub fn get_auction_status(&self) -> AuctionStatus {
        match self.end_time {
            None => AuctionStatus::Ongoing,
            Some(end_time) => {
                let now = Utc::now();
                if now < self.created_at {
                    AuctionStatus::Upcoming
                } else if now < end_time {
                    AuctionStatus::Ongoing
                } else {
                    AuctionStatus::Ended
                }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BidRecord {
    pub id: Uuid,
    pub product_id: Uuid,
    pub user_id: String,
    pub bid_price: f64,
    pub quantity: i64,
    pub status: BidStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BidStatus {
    Pending,
    Confirmed,
    Expired,
    Cancelled,
}

impl BidStatus {
    pub fn as_str(&self) -> &str {
        match self {
            BidStatus::Pending => "Pending",
            BidStatus::Confirmed => "Confirmed",
            BidStatus::Expired => "Expired",
            BidStatus::Cancelled => "Cancelled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Pending" => Some(BidStatus::Pending),
            "Confirmed" => Some(BidStatus::Confirmed),
            "Expired" => Some(BidStatus::Expired),
            "Cancelled" => Some(BidStatus::Cancelled),
            _ => None,
        }
    }
}

impl<DB: sqlx::Database> sqlx::Type<DB> for BidStatus
where
    str: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <str as sqlx::Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <str as sqlx::Type<DB>>::compatible(ty)
    }
}

impl<'q, DB: sqlx::Database> sqlx::Encode<'q, DB> for BidStatus
where
    for<'a> &'a str: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut DB::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as sqlx::Encode<'q, DB>>::encode(self.as_str(), buf)
    }
}

impl<'r, DB: sqlx::Database> sqlx::Decode<'r, DB> for BidStatus
where
    &'r str: sqlx::Decode<'r, DB>,
{
    fn decode(value: DB::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<'r, DB>>::decode(value)?;
        BidStatus::from_str(s).ok_or_else(|| format!("Invalid BidStatus: {}", s).into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceBidRequest {
    pub product_id: Uuid,
    pub user_id: String,
    pub bid_price: f64,
    pub quantity: i64,
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidResponse {
    pub id: Uuid,
    pub product_id: Uuid,
    pub user_id: String,
    pub bid_price: f64,
    pub quantity: i64,
    pub status: BidStatus,
    pub lock_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StockLock {
    pub id: Uuid,
    pub product_id: Uuid,
    pub bid_id: Uuid,
    pub user_id: String,
    pub quantity: i64,
    pub locked_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub status: LockStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LockStatus {
    Active,
    Released,
    Confirmed,
    Expired,
}

impl LockStatus {
    pub fn as_str(&self) -> &str {
        match self {
            LockStatus::Active => "Active",
            LockStatus::Released => "Released",
            LockStatus::Confirmed => "Confirmed",
            LockStatus::Expired => "Expired",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Active" => Some(LockStatus::Active),
            "Released" => Some(LockStatus::Released),
            "Confirmed" => Some(LockStatus::Confirmed),
            "Expired" => Some(LockStatus::Expired),
            _ => None,
        }
    }
}

impl<DB: sqlx::Database> sqlx::Type<DB> for LockStatus
where
    str: sqlx::Type<DB>,
{
    fn type_info() -> DB::TypeInfo {
        <str as sqlx::Type<DB>>::type_info()
    }

    fn compatible(ty: &DB::TypeInfo) -> bool {
        <str as sqlx::Type<DB>>::compatible(ty)
    }
}

impl<'q, DB: sqlx::Database> sqlx::Encode<'q, DB> for LockStatus
where
    for<'a> &'a str: sqlx::Encode<'q, DB>,
{
    fn encode_by_ref(
        &self,
        buf: &mut DB::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <&str as sqlx::Encode<'q, DB>>::encode(self.as_str(), buf)
    }
}

impl<'r, DB: sqlx::Database> sqlx::Decode<'r, DB> for LockStatus
where
    &'r str: sqlx::Decode<'r, DB>,
{
    fn decode(value: DB::ValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <&str as sqlx::Decode<'r, DB>>::decode(value)?;
        LockStatus::from_str(s).ok_or_else(|| format!("Invalid LockStatus: {}", s).into())
    }
}
