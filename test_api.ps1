$BASE_URL = "http://127.0.0.1:3000"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  直播间拍卖商品锁定功能 API 测试脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "1. 健康检查" -ForegroundColor Yellow
try {
    $response = Invoke-RestMethod -Uri "$BASE_URL/health" -Method Get
    Write-Host "   ✓ 服务运行正常: $($response.status)" -ForegroundColor Green
} catch {
    Write-Host "   ✗ 健康检查失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
Write-Host ""

Write-Host "2. 创建拍卖商品" -ForegroundColor Yellow
$productBody = @{
    name = "限量版运动鞋"
    description = "直播间专属限量版运动鞋，原价 999 元"
    total_stock = 10
    start_price = 100.0
    min_increment = 10.0
    room_id = "room_live_001"
} | ConvertTo-Json

try {
    $product = Invoke-RestMethod -Uri "$BASE_URL/products" -Method Post -Body $productBody -ContentType "application/json"
    $productId = $product.id
    Write-Host "   ✓ 商品创建成功" -ForegroundColor Green
    Write-Host "     商品ID: $productId" -ForegroundColor Gray
    Write-Host "     商品名称: $($product.name)" -ForegroundColor Gray
    Write-Host "     起拍价: $($product.start_price) 元" -ForegroundColor Gray
    Write-Host "     库存: $($product.total_stock) 件" -ForegroundColor Gray
} catch {
    Write-Host "   ✗ 创建商品失败: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}
Write-Host ""

Write-Host "3. 查询商品信息" -ForegroundColor Yellow
try {
    $product = Invoke-RestMethod -Uri "$BASE_URL/products/$productId" -Method Get
    Write-Host "   ✓ 查询成功" -ForegroundColor Green
    Write-Host "     当前价格: $($product.current_price) 元" -ForegroundColor Gray
    Write-Host "     可用库存: $($product.available_stock) 件" -ForegroundColor Gray
    Write-Host "     锁定库存: $($product.locked_stock) 件" -ForegroundColor Gray
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "4. 用户出价（库存锁定 15 分钟）" -ForegroundColor Yellow
$bidBody = @{
    product_id = $productId
    user_id = "user_001"
    bid_price = 150.0
    quantity = 2
} | ConvertTo-Json

try {
    $bid = Invoke-RestMethod -Uri "$BASE_URL/bids" -Method Post -Body $bidBody -ContentType "application/json"
    $bidId = $bid.id
    Write-Host "   ✓ 出价成功！库存已锁定 15 分钟" -ForegroundColor Green
    Write-Host "     出价ID: $bidId" -ForegroundColor Gray
    Write-Host "     出价价格: $($bid.bid_price) 元" -ForegroundColor Gray
    Write-Host "     数量: $($bid.quantity) 件" -ForegroundColor Gray
    Write-Host "     锁定到期时间: $($bid.lock_expires_at)" -ForegroundColor Cyan
} catch {
    Write-Host "   ✗ 出价失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "5. 出价后再次查询商品库存" -ForegroundColor Yellow
try {
    $product = Invoke-RestMethod -Uri "$BASE_URL/products/$productId" -Method Get
    Write-Host "   ✓ 查询成功" -ForegroundColor Green
    Write-Host "     当前价格: $($product.current_price) 元" -ForegroundColor Gray
    Write-Host "     可用库存: $($product.available_stock) 件" -ForegroundColor Gray
    Write-Host "     锁定库存: $($product.locked_stock) 件" -ForegroundColor Magenta
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "6. 第二个用户出价（测试价格规则）" -ForegroundColor Yellow
$bidBody2 = @{
    product_id = $productId
    user_id = "user_002"
    bid_price = 155.0
    quantity = 1
} | ConvertTo-Json

try {
    $bid2 = Invoke-RestMethod -Uri "$BASE_URL/bids" -Method Post -Body $bidBody2 -ContentType "application/json"
    Write-Host "   ✓ 出价成功" -ForegroundColor Green
    Write-Host "     用户: user_002, 价格: $($bid2.bid_price) 元, 数量: $($bid2.quantity) 件" -ForegroundColor Gray
} catch {
    Write-Host "   ✗ 出价失败（预期：价格低于最低加价幅度）: $($_.Exception.Message)" -ForegroundColor Yellow
}
Write-Host ""

Write-Host "7. 查询商品锁定记录" -ForegroundColor Yellow
try {
    $locks = Invoke-RestMethod -Uri "$BASE_URL/products/$productId/locks" -Method Get
    Write-Host "   ✓ 查询成功，共 $($locks.Count) 条锁定记录" -ForegroundColor Green
    foreach ($lock in $locks) {
        Write-Host "     - 用户 $($lock.user_id): 锁定 $($lock.quantity) 件, 状态: $($lock.status), 到期: $($lock.expires_at)" -ForegroundColor Gray
    }
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "8. 确认购买（扣除库存）" -ForegroundColor Yellow
$confirmBody = @{
    user_id = "user_001"
} | ConvertTo-Json

try {
    $result = Invoke-RestMethod -Uri "$BASE_URL/bids/$bidId/confirm" -Method Post -Body $confirmBody -ContentType "application/json"
    Write-Host "   ✓ 购买确认成功！库存已正式扣除" -ForegroundColor Green
} catch {
    Write-Host "   ✗ 确认失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "9. 确认购买后查询商品库存" -ForegroundColor Yellow
try {
    $product = Invoke-RestMethod -Uri "$BASE_URL/products/$productId" -Method Get
    Write-Host "   ✓ 查询成功" -ForegroundColor Green
    Write-Host "     总库存: $($product.total_stock) 件" -ForegroundColor Gray
    Write-Host "     可用库存: $($product.available_stock) 件" -ForegroundColor Gray
    Write-Host "     锁定库存: $($product.locked_stock) 件" -ForegroundColor Gray
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "10. 模拟网络波动重复出价（旧锁定自动释放）" -ForegroundColor Yellow
$dupBidBody = @{
    product_id = $productId
    user_id = "user_001"
    bid_price = 200.0
    quantity = 3
} | ConvertTo-Json

try {
    $dupBid = Invoke-RestMethod -Uri "$BASE_URL/bids" -Method Post -Body $dupBidBody -ContentType "application/json"
    Write-Host "   ✓ 重复出价成功！旧锁定已自动释放" -ForegroundColor Green
    Write-Host "     新出价ID: $($dupBid.id)" -ForegroundColor Gray
    Write-Host "     新出价价格: $($dupBid.bid_price) 元" -ForegroundColor Gray
    Write-Host "     数量: $($dupBid.quantity) 件" -ForegroundColor Gray
} catch {
    Write-Host "   ✗ 出价失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "11. 重复出价后验证库存（只有最新出价在锁定）" -ForegroundColor Yellow
try {
    $product = Invoke-RestMethod -Uri "$BASE_URL/products/$productId" -Method Get
    Write-Host "   ✓ 查询成功" -ForegroundColor Green
    Write-Host "     总库存: $($product.total_stock) 件" -ForegroundColor Gray
    Write-Host "     可用库存: $($product.available_stock) 件" -ForegroundColor Gray
    Write-Host "     锁定库存: $($product.locked_stock) 件" -ForegroundColor Magenta
    Write-Host "     说明: 锁定库存应为3件（最新出价），而非5件（双重锁定）" -ForegroundColor Cyan
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "12. 使用幂等键重复出价（返回同一结果）" -ForegroundColor Yellow
$idempotencyKey = "request-" + [guid]::NewGuid().ToString()
$idemBidBody1 = @{
    product_id = $productId
    user_id = "user_003"
    bid_price = 180.0
    quantity = 1
    idempotency_key = $idempotencyKey
} | ConvertTo-Json

try {
    $bid1 = Invoke-RestMethod -Uri "$BASE_URL/bids" -Method Post -Body $idemBidBody1 -ContentType "application/json"
    Write-Host "   第一次出价ID: $($bid1.id)" -ForegroundColor Gray
    
    $idemBidBody2 = @{
        product_id = $productId
        user_id = "user_003"
        bid_price = 250.0
        quantity = 5
        idempotency_key = $idempotencyKey
    } | ConvertTo-Json
    
    $bid2 = Invoke-RestMethod -Uri "$BASE_URL/bids" -Method Post -Body $idemBidBody2 -ContentType "application/json"
    Write-Host "   第二次出价ID: $($bid2.id)" -ForegroundColor Gray
    
    if ($bid1.id -eq $bid2.id) {
        Write-Host "   ✓ 幂等键生效！两次请求返回同一出价结果" -ForegroundColor Green
        Write-Host "     价格: $($bid1.bid_price) 元, 数量: $($bid1.quantity) 件" -ForegroundColor Gray
    } else {
        Write-Host "   ⚠ 幂等键未生效，返回了不同的出价" -ForegroundColor Yellow
    }
} catch {
    Write-Host "   ✗ 出价失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "13. 幂等键测试后验证库存（没有重复锁定）" -ForegroundColor Yellow
try {
    $product = Invoke-RestMethod -Uri "$BASE_URL/products/$productId" -Method Get
    Write-Host "   ✓ 查询成功" -ForegroundColor Green
    Write-Host "     总库存: $($product.total_stock) 件" -ForegroundColor Gray
    Write-Host "     可用库存: $($product.available_stock) 件" -ForegroundColor Gray
    Write-Host "     锁定库存: $($product.locked_stock) 件" -ForegroundColor Magenta
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "14. 查询商品锁定记录详情" -ForegroundColor Yellow
try {
    $locks = Invoke-RestMethod -Uri "$BASE_URL/products/$productId/locks" -Method Get
    Write-Host "   ✓ 查询成功，共 $($locks.Count) 条锁定记录" -ForegroundColor Green
    $activeCount = ($locks | Where-Object { $_.status -eq "Active" }).Count
    Write-Host "     活动锁定: $activeCount 条" -ForegroundColor Cyan
    foreach ($lock in $locks) {
        $statusColor = if ($lock.status -eq "Active") { "Green" } else { "Gray" }
        Write-Host "     - 用户 $($lock.user_id): $($lock.quantity) 件, 状态: $($lock.status)" -ForegroundColor $statusColor
    }
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  测试完成！" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "防重复出价机制说明:" -ForegroundColor White
Write-Host "  1. 同一用户同一商品只能有一个活动锁定" -ForegroundColor Gray
Write-Host "  2. 新出价自动释放旧锁定，防止双重锁定" -ForegroundColor Gray
Write-Host "  3. 支持幂等键，相同请求ID返回同一结果" -ForegroundColor Gray
Write-Host "  4. 所有操作在事务内完成，保证原子性" -ForegroundColor Gray
