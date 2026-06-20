use auction_lock_service::db;
use auction_lock_service::models::*;
use auction_lock_service::services::AuctionService;
use chrono::Duration;
use sqlx::SqlitePool;
use uuid::Uuid;

async fn setup_test_db() -> SqlitePool {
    let pool = db::init_db("sqlite::memory:").await.unwrap();
    pool
}

#[tokio::test]
async fn test_create_product() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();
    assert_eq!(product.name, "测试商品");
    assert_eq!(product.total_stock, 100);
    assert_eq!(product.available_stock, 100);
    assert_eq!(product.locked_stock, 0);
    assert_eq!(product.current_price, 10.0);
    assert_eq!(product.room_id, "room_001");
}

#[tokio::test]
async fn test_place_bid_success() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
    };

    let bid = service.place_bid(&bid_req).await.unwrap();
    assert_eq!(bid.bid_price, 15.0);
    assert_eq!(bid.quantity, 2);
    assert_eq!(bid.status, BidStatus::Confirmed);

    let expected_expiry = chrono::Utc::now() + Duration::minutes(db::LOCK_DURATION_MINUTES);
    assert!(bid.lock_expires_at > chrono::Utc::now());
    assert!(bid.lock_expires_at <= expected_expiry + Duration::seconds(5));

    let updated_product = service.get_product(product.id).await.unwrap();
    assert_eq!(updated_product.available_stock, 98);
    assert_eq!(updated_product.locked_stock, 2);
    assert_eq!(updated_product.current_price, 15.0);
}

#[tokio::test]
async fn test_place_bid_insufficient_stock() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 5,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 10,
    };

    let result = service.place_bid(&bid_req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_place_bid_price_too_low() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 5.0,
        quantity: 1,
    };

    let result = service.place_bid(&bid_req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_place_bid_invalid_increment() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 5.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 12.0,
        quantity: 1,
    };

    let result = service.place_bid(&bid_req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_confirm_purchase() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 3,
    };

    let bid = service.place_bid(&bid_req).await.unwrap();

    service.confirm_purchase(bid.id, "user_001").await.unwrap();

    let updated_product = service.get_product(product.id).await.unwrap();
    assert_eq!(updated_product.total_stock, 97);
    assert_eq!(updated_product.locked_stock, 0);
    assert_eq!(updated_product.available_stock, 97);
}

#[tokio::test]
async fn test_cancel_bid() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 3,
    };

    let bid = service.place_bid(&bid_req).await.unwrap();

    let product_after_bid = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after_bid.available_stock, 97);
    assert_eq!(product_after_bid.locked_stock, 3);

    service.cancel_bid(bid.id, "user_001").await.unwrap();

    let updated_product = service.get_product(product.id).await.unwrap();
    assert_eq!(updated_product.available_stock, 100);
    assert_eq!(updated_product.locked_stock, 0);
    assert_eq!(updated_product.total_stock, 100);
}

#[tokio::test]
async fn test_expired_locks_cleanup() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "这是一个测试商品".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 5,
    };

    let bid = service.place_bid(&bid_req).await.unwrap();

    let product_after_bid = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after_bid.available_stock, 95);
    assert_eq!(product_after_bid.locked_stock, 5);

    sqlx::query(
        r#"
        UPDATE stock_locks SET expires_at = ? WHERE bid_id = ?
        "#,
    )
    .bind((chrono::Utc::now() - Duration::hours(1)).to_rfc3339())
    .bind(bid.id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let cleaned = service.cleanup_expired_locks().await.unwrap();
    assert_eq!(cleaned, 1);

    let updated_product = service.get_product(product.id).await.unwrap();
    assert_eq!(updated_product.available_stock, 100);
    assert_eq!(updated_product.locked_stock, 0);
}

#[tokio::test]
async fn test_concurrent_bids() {
    let pool = setup_test_db().await;
    let service = std::sync::Arc::new(AuctionService::new(pool.clone()));

    let req = CreateProductRequest {
        name: "限量商品".to_string(),
        description: "限量商品，库存紧张".to_string(),
        total_stock: 5,
        start_price: 100.0,
        min_increment: 10.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let mut handles = Vec::new();
    for i in 0..10 {
        let service = service.clone();
        let product_id = product.id;
        let handle = tokio::spawn(async move {
            let bid_req = PlaceBidRequest {
                product_id,
                user_id: format!("user_{:03}", i),
                bid_price: 100.0 + (i as f64) * 10.0,
                quantity: 1,
            };
            service.place_bid(&bid_req).await
        });
        handles.push(handle);
    }

    let results = futures::future::join_all(handles).await;
    let success_count = results
        .iter()
        .filter(|r| r.as_ref().unwrap().is_ok())
        .count();

    assert_eq!(success_count, 5);

    let updated_product = service.get_product(product.id).await.unwrap();
    assert_eq!(updated_product.available_stock, 0);
    assert_eq!(updated_product.locked_stock, 5);
    assert_eq!(updated_product.total_stock, 5);
}

#[tokio::test]
async fn test_get_products_by_room() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    for i in 0..3 {
        let req = CreateProductRequest {
            name: format!("商品{}", i),
            description: format!("描述{}", i),
            total_stock: 10,
            start_price: 10.0,
            min_increment: 1.0,
            room_id: "room_001".to_string(),
        };
        service.create_product(&req).await.unwrap();
    }

    let req = CreateProductRequest {
        name: "其他房间商品".to_string(),
        description: "其他房间".to_string(),
        total_stock: 10,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_002".to_string(),
    };
    service.create_product(&req).await.unwrap();

    let products = service.get_products_by_room("room_001").await.unwrap();
    assert_eq!(products.len(), 3);

    let products = service.get_products_by_room("room_002").await.unwrap();
    assert_eq!(products.len(), 1);
}
