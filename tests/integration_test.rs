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

#[tokio::test]
async fn test_duplicate_bid_releases_old_lock() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "测试重复出价自动释放旧锁定".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req1 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 3,
        idempotency_key: None,
    };

    let bid1 = service.place_bid(&bid_req1).await.unwrap();
    assert_eq!(bid1.quantity, 3);

    let product_after_first = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after_first.available_stock, 97);
    assert_eq!(product_after_first.locked_stock, 3);

    let bid_req2 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 25.0,
        quantity: 5,
        idempotency_key: None,
    };

    let bid2 = service.place_bid(&bid_req2).await.unwrap();
    assert_eq!(bid2.bid_price, 25.0);
    assert_eq!(bid2.quantity, 5);
    assert_ne!(bid1.id, bid2.id);

    let product_after_second = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after_second.available_stock, 95);
    assert_eq!(product_after_second.locked_stock, 5);
    assert_eq!(product_after_second.total_stock, 100);

    let locks = service.get_product_locks(product.id).await.unwrap();
    let active_locks: Vec<_> = locks
        .iter()
        .filter(|l| l.status == LockStatus::Active)
        .collect();
    assert_eq!(active_locks.len(), 1);
    assert_eq!(active_locks[0].bid_id, bid2.id);
}

#[tokio::test]
async fn test_rapid_repeated_bids_no_double_lock() {
    let pool = setup_test_db().await;
    let service = std::sync::Arc::new(AuctionService::new(pool.clone()));

    let req = CreateProductRequest {
        name: "热门商品".to_string(),
        description: "模拟网络波动快速重复点击".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let mut handles = Vec::new();
    for i in 0..5 {
        let service = service.clone();
        let product_id = product.id;
        let handle = tokio::spawn(async move {
            let bid_req = PlaceBidRequest {
                product_id,
                user_id: "user_flaky".to_string(),
                bid_price: 20.0 + (i as f64) * 5.0,
                quantity: 2,
                idempotency_key: None,
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

    assert!(success_count >= 1);

    let final_product = service.get_product(product.id).await.unwrap();
    assert_eq!(final_product.locked_stock, 2);
    assert_eq!(final_product.available_stock, 98);
    assert_eq!(final_product.total_stock, 100);

    let locks = service.get_product_locks(product.id).await.unwrap();
    let user_locks: Vec<_> = locks
        .iter()
        .filter(|l| l.user_id == "user_flaky" && l.status == LockStatus::Active)
        .collect();
    assert_eq!(user_locks.len(), 1);
}

#[tokio::test]
async fn test_idempotency_key_same_result() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "测试幂等键".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let idempotency_key = "req-abc-123".to_string();

    let bid_req1 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
        idempotency_key: Some(idempotency_key.clone()),
    };

    let bid1 = service.place_bid(&bid_req1).await.unwrap();

    let bid_req2 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 20.0,
        quantity: 5,
        idempotency_key: Some(idempotency_key.clone()),
    };

    let bid2 = service.place_bid(&bid_req2).await.unwrap();

    assert_eq!(bid1.id, bid2.id);
    assert_eq!(bid1.bid_price, bid2.bid_price);
    assert_eq!(bid1.quantity, bid2.quantity);

    let final_product = service.get_product(product.id).await.unwrap();
    assert_eq!(final_product.locked_stock, 2);
    assert_eq!(final_product.available_stock, 98);
}

#[tokio::test]
async fn test_different_users_both_get_locks() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "不同用户各自获得锁定".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req1 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
        idempotency_key: None,
    };
    service.place_bid(&bid_req1).await.unwrap();

    let bid_req2 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_002".to_string(),
        bid_price: 20.0,
        quantity: 3,
        idempotency_key: None,
    };
    service.place_bid(&bid_req2).await.unwrap();

    let final_product = service.get_product(product.id).await.unwrap();
    assert_eq!(final_product.locked_stock, 5);
    assert_eq!(final_product.available_stock, 95);

    let locks = service.get_product_locks(product.id).await.unwrap();
    let active_locks: Vec<_> = locks
        .iter()
        .filter(|l| l.status == LockStatus::Active)
        .collect();
    assert_eq!(active_locks.len(), 2);
}

#[tokio::test]
async fn test_user_bids_on_multiple_products() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    for i in 0..2 {
        let req = CreateProductRequest {
            name: format!("商品{}", i),
            description: format!("商品{}描述", i),
            total_stock: 50,
            start_price: 10.0,
            min_increment: 1.0,
            room_id: "room_001".to_string(),
        };
        service.create_product(&req).await.unwrap();
    }

    let products = service.get_products_by_room("room_001").await.unwrap();
    assert_eq!(products.len(), 2);

    for product in &products {
        let bid_req = PlaceBidRequest {
            product_id: product.id,
            user_id: "user_001".to_string(),
            bid_price: 15.0,
            quantity: 2,
            idempotency_key: None,
        };
        service.place_bid(&bid_req).await.unwrap();
    }

    for product in &products {
        let p = service.get_product(product.id).await.unwrap();
        assert_eq!(p.locked_stock, 2);
        assert_eq!(p.available_stock, 48);
    }
}

#[tokio::test]
async fn test_create_product_with_auction_duration() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "限时拍卖商品".to_string(),
        description: "30分钟后自动结束".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(30),
    };

    let product = service.create_product(&req).await.unwrap();
    assert_eq!(product.name, "限时拍卖商品");
    assert!(product.end_time.is_some());
    assert_eq!(product.auction_status, AuctionStatus::Ongoing);

    let end_time = product.end_time.unwrap();
    let expected_end = chrono::Utc::now() + chrono::Duration::minutes(30);
    assert!((end_time - expected_end).num_seconds().abs() < 5);
}

#[tokio::test]
async fn test_create_product_without_duration_no_end_time() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "常规商品".to_string(),
        description: "无时间限制".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: None,
    };

    let product = service.create_product(&req).await.unwrap();
    assert!(product.end_time.is_none());
    assert_eq!(product.auction_status, AuctionStatus::Ongoing);
}

#[tokio::test]
async fn test_auction_status_transitions() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "测试状态变化".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(60),
    };

    let product = service.create_product(&req).await.unwrap();
    assert_eq!(product.auction_status, AuctionStatus::Ongoing);

    let status = service.get_auction_status(product.id).await.unwrap();
    assert_eq!(status, AuctionStatus::Ongoing);

    sqlx::query(
        r#"
        UPDATE products SET end_time = ? WHERE id = ?
        "#,
    )
    .bind((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339())
    .bind(product.id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let status = service.get_auction_status(product.id).await.unwrap();
    assert_eq!(status, AuctionStatus::Ended);
}

#[tokio::test]
async fn test_cannot_bid_on_ended_auction() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "已结束拍卖".to_string(),
        description: "测试".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(60),
    };

    let product = service.create_product(&req).await.unwrap();

    sqlx::query(
        r#"
        UPDATE products SET end_time = ? WHERE id = ?
        "#,
    )
    .bind((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339())
    .bind(product.id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let bid_req = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 1,
        idempotency_key: None,
    };

    let result = service.place_bid(&bid_req).await;
    assert!(result.is_err());

    let product_after = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after.locked_stock, 0);
    assert_eq!(product_after.available_stock, 100);
}

#[tokio::test]
async fn test_auction_ended_auto_release_locks() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "即将结束的拍卖".to_string(),
        description: "测试".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(60),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req1 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
        idempotency_key: None,
    };
    service.place_bid(&bid_req1).await.unwrap();

    let bid_req2 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_002".to_string(),
        bid_price: 20.0,
        quantity: 3,
        idempotency_key: None,
    };
    service.place_bid(&bid_req2).await.unwrap();

    let product_after_bids = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after_bids.locked_stock, 5);
    assert_eq!(product_after_bids.available_stock, 95);

    sqlx::query(
        r#"
        UPDATE products SET end_time = ? WHERE id = ?
        "#,
    )
    .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
    .bind(product.id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let released = service.cleanup_ended_auctions().await.unwrap();
    assert_eq!(released, 5);

    let product_final = service.get_product(product.id).await.unwrap();
    assert_eq!(product_final.locked_stock, 0);
    assert_eq!(product_final.available_stock, 100);
    assert_eq!(product_final.total_stock, 100);
}

#[tokio::test]
async fn test_ended_auction_with_confirmed_purchases() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "测试".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(60),
    };

    let product = service.create_product(&req).await.unwrap();

    let bid_req1 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
        idempotency_key: None,
    };
    let bid1 = service.place_bid(&bid_req1).await.unwrap();

    let bid_req2 = PlaceBidRequest {
        product_id: product.id,
        user_id: "user_002".to_string(),
        bid_price: 20.0,
        quantity: 3,
        idempotency_key: None,
    };
    let bid2 = service.place_bid(&bid_req2).await.unwrap();

    service.confirm_purchase(bid1.id, "user_001").await.unwrap();

    let product_after_confirm = service.get_product(product.id).await.unwrap();
    assert_eq!(product_after_confirm.total_stock, 98);
    assert_eq!(product_after_confirm.locked_stock, 3);
    assert_eq!(product_after_confirm.available_stock, 95);

    sqlx::query(
        r#"
        UPDATE products SET end_time = ? WHERE id = ?
        "#,
    )
    .bind((chrono::Utc::now() - chrono::Duration::minutes(1)).to_rfc3339())
    .bind(product.id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let released = service.cleanup_ended_auctions().await.unwrap();
    assert_eq!(released, 3);

    let product_final = service.get_product(product.id).await.unwrap();
    assert_eq!(product_final.total_stock, 98);
    assert_eq!(product_final.locked_stock, 0);
    assert_eq!(product_final.available_stock, 98);
}

#[tokio::test]
async fn test_get_ended_auctions_with_active_locks() {
    use auction_lock_service::db;

    let pool = setup_test_db().await;
    let service = AuctionService::new(pool.clone());

    let req1 = CreateProductRequest {
        name: "已结束商品1".to_string(),
        description: "测试".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(60),
    };
    let product1 = service.create_product(&req1).await.unwrap();

    let req2 = CreateProductRequest {
        name: "进行中商品".to_string(),
        description: "测试".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(60),
    };
    let product2 = service.create_product(&req2).await.unwrap();

    let bid_req1 = PlaceBidRequest {
        product_id: product1.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
        idempotency_key: None,
    };
    service.place_bid(&bid_req1).await.unwrap();

    let bid_req2 = PlaceBidRequest {
        product_id: product2.id,
        user_id: "user_001".to_string(),
        bid_price: 15.0,
        quantity: 2,
        idempotency_key: None,
    };
    service.place_bid(&bid_req2).await.unwrap();

    sqlx::query(
        r#"
        UPDATE products SET end_time = ? WHERE id = ?
        "#,
    )
    .bind((chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339())
    .bind(product1.id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let ended_auctions = db::get_ended_auctions_with_active_locks(&pool)
        .await
        .unwrap();
    assert_eq!(ended_auctions.len(), 1);
    assert_eq!(ended_auctions[0].id, product1.id);
}

#[tokio::test]
async fn test_invalid_auction_duration_rejected() {
    let pool = setup_test_db().await;
    let service = AuctionService::new(pool);

    let req = CreateProductRequest {
        name: "测试商品".to_string(),
        description: "测试".to_string(),
        total_stock: 100,
        start_price: 10.0,
        min_increment: 1.0,
        room_id: "room_001".to_string(),
        auction_duration_minutes: Some(-30),
    };

    let result = service.create_product(&req).await;
    assert!(result.is_err());
}
