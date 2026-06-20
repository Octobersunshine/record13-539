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

Write-Host "10. 查询直播间商品列表" -ForegroundColor Yellow
try {
    $products = Invoke-RestMethod -Uri "$BASE_URL/products/room/room_live_001" -Method Get
    Write-Host "   ✓ 查询成功，直播间共 $($products.Count) 件商品" -ForegroundColor Green
    foreach ($p in $products) {
        Write-Host "     - $($p.name): 当前价 $($p.current_price) 元, 库存 $($p.available_stock)/$($p.total_stock)" -ForegroundColor Gray
    }
} catch {
    Write-Host "   ✗ 查询失败: $($_.Exception.Message)" -ForegroundColor Red
}
Write-Host ""

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  测试完成！" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
