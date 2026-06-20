use chrono::{Duration, Utc};
use sqlx::{
    sqlite::{Sqlite, SqlitePoolOptions},
    Executor, SqlitePool,
};
use uuid::Uuid;

use crate::errors::AppResult;
use crate::models::*;

pub const LOCK_DURATION_MINUTES: i64 = 15;

pub async fn init_db(database_url: &str) -> AppResult<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS products (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT NOT NULL,
            total_stock INTEGER NOT NULL,
            available_stock INTEGER NOT NULL,
            locked_stock INTEGER NOT NULL DEFAULT 0,
            start_price REAL NOT NULL,
            current_price REAL NOT NULL,
            min_increment REAL NOT NULL,
            room_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bid_records (
            id TEXT PRIMARY KEY,
            product_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            bid_price REAL NOT NULL,
            quantity INTEGER NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS stock_locks (
            id TEXT PRIMARY KEY,
            product_id TEXT NOT NULL,
            bid_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            locked_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            status TEXT NOT NULL,
            FOREIGN KEY (product_id) REFERENCES products(id),
            FOREIGN KEY (bid_id) REFERENCES bid_records(id)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_stock_locks_expires_at ON stock_locks(expires_at)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_stock_locks_status ON stock_locks(status)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_stock_locks_product_user 
        ON stock_locks(product_id, user_id, status)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS idempotency_keys (
            key TEXT PRIMARY KEY,
            bid_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn create_product(pool: &SqlitePool, req: &CreateProductRequest) -> AppResult<Product> {
    let now = Utc::now();
    let product = Product {
        id: Uuid::new_v4(),
        name: req.name.clone(),
        description: req.description.clone(),
        total_stock: req.total_stock,
        available_stock: req.total_stock,
        locked_stock: 0,
        start_price: req.start_price,
        current_price: req.start_price,
        min_increment: req.min_increment,
        room_id: req.room_id.clone(),
        created_at: now,
        updated_at: now,
    };

    sqlx::query(
        r#"
        INSERT INTO products (id, name, description, total_stock, available_stock, locked_stock,
                              start_price, current_price, min_increment, room_id, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(product.id.to_string())
    .bind(&product.name)
    .bind(&product.description)
    .bind(product.total_stock)
    .bind(product.available_stock)
    .bind(product.locked_stock)
    .bind(product.start_price)
    .bind(product.current_price)
    .bind(product.min_increment)
    .bind(&product.room_id)
    .bind(product.created_at.to_rfc3339())
    .bind(product.updated_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(product)
}

pub async fn get_product(pool: &SqlitePool, product_id: Uuid) -> AppResult<Option<Product>> {
    let product: Option<Product> = sqlx::query_as::<_, Product>(
        r#"
        SELECT * FROM products WHERE id = ?
        "#,
    )
    .bind(product_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(product)
}

pub async fn get_products_by_room(pool: &SqlitePool, room_id: &str) -> AppResult<Vec<Product>> {
    let products: Vec<Product> = sqlx::query_as::<_, Product>(
        r#"
        SELECT * FROM products WHERE room_id = ? ORDER BY created_at DESC
        "#,
    )
    .bind(room_id)
    .fetch_all(pool)
    .await?;

    Ok(products)
}

pub async fn update_product_price<'c, E>(
    executor: E,
    product_id: Uuid,
    new_price: f64,
) -> AppResult<()>
where
    E: Executor<'c, Database = Sqlite>,
{
    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE products SET current_price = ?, updated_at = ? WHERE id = ?
        "#,
    )
    .bind(new_price)
    .bind(now.to_rfc3339())
    .bind(product_id.to_string())
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn lock_stock<'c, E>(
    executor: E,
    product_id: Uuid,
    bid_id: Uuid,
    user_id: &str,
    quantity: i64,
) -> AppResult<StockLock>
where
    E: Executor<'c, Database = Sqlite>,
{
    let now = Utc::now();
    let expires_at = now + Duration::minutes(LOCK_DURATION_MINUTES);

    let lock = StockLock {
        id: Uuid::new_v4(),
        product_id,
        bid_id,
        user_id: user_id.to_string(),
        quantity,
        locked_at: now,
        expires_at,
        status: LockStatus::Active,
    };

    sqlx::query(
        r#"
        UPDATE products 
        SET available_stock = available_stock - ?,
            locked_stock = locked_stock + ?,
            updated_at = ?
        WHERE id = ? AND available_stock >= ?
        "#,
    )
    .bind(quantity)
    .bind(quantity)
    .bind(now.to_rfc3339())
    .bind(product_id.to_string())
    .bind(quantity)
    .execute(executor)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO stock_locks (id, product_id, bid_id, user_id, quantity, locked_at, expires_at, status)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(lock.id.to_string())
    .bind(lock.product_id.to_string())
    .bind(lock.bid_id.to_string())
    .bind(&lock.user_id)
    .bind(lock.quantity)
    .bind(lock.locked_at.to_rfc3339())
    .bind(lock.expires_at.to_rfc3339())
    .bind(&lock.status)
    .execute(executor)
    .await?;

    Ok(lock)
}

pub async fn create_bid<'c, E>(
    executor: E,
    product_id: Uuid,
    user_id: &str,
    bid_price: f64,
    quantity: i64,
) -> AppResult<BidRecord>
where
    E: Executor<'c, Database = Sqlite>,
{
    let now = Utc::now();
    let bid = BidRecord {
        id: Uuid::new_v4(),
        product_id,
        user_id: user_id.to_string(),
        bid_price,
        quantity,
        status: BidStatus::Pending,
        created_at: now,
    };

    sqlx::query(
        r#"
        INSERT INTO bid_records (id, product_id, user_id, bid_price, quantity, status, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(bid.id.to_string())
    .bind(bid.product_id.to_string())
    .bind(&bid.user_id)
    .bind(bid.bid_price)
    .bind(bid.quantity)
    .bind(&bid.status)
    .bind(bid.created_at.to_rfc3339())
    .execute(executor)
    .await?;

    Ok(bid)
}

pub async fn get_bid(pool: &SqlitePool, bid_id: Uuid) -> AppResult<Option<BidRecord>> {
    let bid: Option<BidRecord> = sqlx::query_as::<_, BidRecord>(
        r#"
        SELECT * FROM bid_records WHERE id = ?
        "#,
    )
    .bind(bid_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(bid)
}

pub async fn update_bid_status<'c, E>(executor: E, bid_id: Uuid, status: BidStatus) -> AppResult<()>
where
    E: Executor<'c, Database = Sqlite>,
{
    sqlx::query(
        r#"
        UPDATE bid_records SET status = ? WHERE id = ?
        "#,
    )
    .bind(&status)
    .bind(bid_id.to_string())
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn get_active_lock_by_bid(
    pool: &SqlitePool,
    bid_id: Uuid,
) -> AppResult<Option<StockLock>> {
    let lock: Option<StockLock> = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks 
        WHERE bid_id = ? AND status = 'Active'
        "#,
    )
    .bind(bid_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(lock)
}

pub async fn confirm_lock(pool: &SqlitePool, lock_id: Uuid) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let lock: StockLock = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks WHERE id = ? AND status = 'Active'
        "#,
    )
    .bind(lock_id.to_string())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE stock_locks SET status = 'Confirmed' WHERE id = ?
        "#,
    )
    .bind(lock_id.to_string())
    .execute(&mut *tx)
    .await?;

    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE products 
        SET total_stock = total_stock - ?,
            locked_stock = locked_stock - ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(lock.quantity)
    .bind(lock.quantity)
    .bind(now.to_rfc3339())
    .bind(lock.product_id.to_string())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn release_lock(pool: &SqlitePool, lock_id: Uuid) -> AppResult<()> {
    let mut tx = pool.begin().await?;

    let lock: StockLock = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks WHERE id = ? AND status = 'Active'
        "#,
    )
    .bind(lock_id.to_string())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE stock_locks SET status = 'Released' WHERE id = ?
        "#,
    )
    .bind(lock_id.to_string())
    .execute(&mut *tx)
    .await?;

    let now = Utc::now();
    sqlx::query(
        r#"
        UPDATE products 
        SET available_stock = available_stock + ?,
            locked_stock = locked_stock - ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(lock.quantity)
    .bind(lock.quantity)
    .bind(now.to_rfc3339())
    .bind(lock.product_id.to_string())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn expire_old_locks(pool: &SqlitePool) -> AppResult<i64> {
    let now = Utc::now();

    let expired_locks: Vec<StockLock> = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks 
        WHERE status = 'Active' AND expires_at < ?
        "#,
    )
    .bind(now.to_rfc3339())
    .fetch_all(pool)
    .await?;

    let count = expired_locks.len() as i64;

    for lock in expired_locks {
        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE stock_locks SET status = 'Expired' WHERE id = ?
            "#,
        )
        .bind(lock.id.to_string())
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE bid_records SET status = 'Expired' WHERE id = ?
            "#,
        )
        .bind(lock.bid_id.to_string())
        .execute(&mut *tx)
        .await?;

        let now_str = now.to_rfc3339();
        sqlx::query(
            r#"
            UPDATE products 
            SET available_stock = available_stock + ?,
                locked_stock = locked_stock - ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(lock.quantity)
        .bind(lock.quantity)
        .bind(now_str)
        .bind(lock.product_id.to_string())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        tracing::info!(
            "Expired lock {} for product {}, quantity {}",
            lock.id,
            lock.product_id,
            lock.quantity
        );
    }

    Ok(count)
}

pub async fn get_product_locks(pool: &SqlitePool, product_id: Uuid) -> AppResult<Vec<StockLock>> {
    let locks: Vec<StockLock> = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks 
        WHERE product_id = ? 
        ORDER BY locked_at DESC
        "#,
    )
    .bind(product_id.to_string())
    .fetch_all(pool)
    .await?;

    Ok(locks)
}

pub async fn get_active_locks_by_user_and_product(
    pool: &SqlitePool,
    product_id: Uuid,
    user_id: &str,
) -> AppResult<Vec<StockLock>> {
    let locks: Vec<StockLock> = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks 
        WHERE product_id = ? AND user_id = ? AND status = 'Active'
        ORDER BY locked_at DESC
        "#,
    )
    .bind(product_id.to_string())
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(locks)
}

pub async fn release_user_active_locks<'c, E>(
    executor: &'c mut E,
    product_id: Uuid,
    user_id: &str,
) -> AppResult<i64>
where
    E: sqlx::Executor<'c, Database = Sqlite> + 'c,
    for<'e> &'e mut E: sqlx::Executor<'e, Database = Sqlite>,
{
    let active_locks: Vec<StockLock> = sqlx::query_as::<_, StockLock>(
        r#"
        SELECT * FROM stock_locks 
        WHERE product_id = ? AND user_id = ? AND status = 'Active'
        "#,
    )
    .bind(product_id.to_string())
    .bind(user_id)
    .fetch_all(&mut *executor)
    .await?;

    let count = active_locks.len() as i64;

    for lock in active_locks {
        sqlx::query(
            r#"
            UPDATE stock_locks SET status = 'Released' WHERE id = ?
            "#,
        )
        .bind(lock.id.to_string())
        .execute(&mut *executor)
        .await?;

        sqlx::query(
            r#"
            UPDATE bid_records SET status = 'Cancelled' WHERE id = ?
            "#,
        )
        .bind(lock.bid_id.to_string())
        .execute(&mut *executor)
        .await?;

        sqlx::query(
            r#"
            UPDATE products 
            SET available_stock = available_stock + ?,
                locked_stock = locked_stock - ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(lock.quantity)
        .bind(lock.quantity)
        .bind(Utc::now().to_rfc3339())
        .bind(lock.product_id.to_string())
        .execute(&mut *executor)
        .await?;
    }

    if count > 0 {
        tracing::info!(
            "Released {} active locks for user {} on product {}",
            count,
            user_id,
            product_id
        );
    }

    Ok(count)
}

pub async fn get_idempotency_result(
    pool: &SqlitePool,
    idempotency_key: &str,
) -> AppResult<Option<Uuid>> {
    let result: Option<(String,)> = sqlx::query_as(
        r#"
        SELECT bid_id FROM idempotency_keys 
        WHERE key = ? AND expires_at > ?
        "#,
    )
    .bind(idempotency_key)
    .bind(Utc::now().to_rfc3339())
    .fetch_optional(pool)
    .await?;

    Ok(result.and_then(|(bid_id,)| Uuid::parse_str(&bid_id).ok()))
}

pub async fn set_idempotency_result<'c, E>(
    executor: E,
    idempotency_key: &str,
    bid_id: Uuid,
) -> AppResult<()>
where
    E: Executor<'c, Database = Sqlite>,
{
    let now = Utc::now();
    let expires_at = now + Duration::hours(24);

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO idempotency_keys (key, bid_id, created_at, expires_at)
        VALUES (?, ?, ?, ?)
        "#,
    )
    .bind(idempotency_key)
    .bind(bid_id.to_string())
    .bind(now.to_rfc3339())
    .bind(expires_at.to_rfc3339())
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn cleanup_expired_idempotency_keys(pool: &SqlitePool) -> AppResult<i64> {
    let result = sqlx::query(
        r#"
        DELETE FROM idempotency_keys WHERE expires_at < ?
        "#,
    )
    .bind(Utc::now().to_rfc3339())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as i64)
}
