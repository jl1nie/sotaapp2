#!/bin/bash
# デプロイ後の疎通確認スクリプト

# set -e は使わない（個別チェックで失敗しても続行する）

BASE_URL="${1:-https://sotaapp2.fly.dev}"
TIMEOUT=5
declare -i FAILED=0
declare -i PASSED=0

echo "=== デプロイ後疎通確認 ==="
echo "対象: $BASE_URL"
echo ""

check_endpoint() {
    local name="$1"
    local path="$2"
    local expected_status="${3:-200}"
    local check_json="${4:-false}"

    local url="${BASE_URL}${path}"
    local response
    local status

    response=$(curl -s -w "\n%{http_code}" --max-time $TIMEOUT "$url" 2>/dev/null) || {
        echo "❌ [$name] タイムアウト: $url"
        ((FAILED++))
        return 1
    }

    status=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')

    if [ "$status" != "$expected_status" ]; then
        echo "❌ [$name] ステータス: $status (期待: $expected_status)"
        ((FAILED++))
        return 1
    fi

    if [ "$check_json" = "true" ]; then
        if ! echo "$body" | jq . >/dev/null 2>&1; then
            echo "❌ [$name] 無効なJSON"
            ((FAILED++))
            return 1
        fi
    fi

    echo "✅ [$name] OK"
    ((PASSED++))
    return 0
}

check_json_field() {
    local name="$1"
    local path="$2"
    local field="$3"

    local url="${BASE_URL}${path}"
    local response

    response=$(curl -s --max-time $TIMEOUT "$url" 2>/dev/null) || {
        echo "❌ [$name] タイムアウト"
        ((FAILED++))
        return 1
    }

    if ! echo "$response" | jq -e ".$field" >/dev/null 2>&1; then
        echo "❌ [$name] フィールド '$field' が存在しない"
        ((FAILED++))
        return 1
    fi

    echo "✅ [$name] OK"
    ((PASSED++))
    return 0
}

check_array_endpoint() {
    local name="$1"
    local path="$2"

    local url="${BASE_URL}${path}"
    local response

    response=$(curl -s --max-time $TIMEOUT "$url" 2>/dev/null) || {
        echo "❌ [$name] タイムアウト"
        ((FAILED++))
        return 1
    }

    if ! echo "$response" | jq -e 'type == "array"' >/dev/null 2>&1; then
        echo "❌ [$name] 配列ではない"
        ((FAILED++))
        return 1
    fi

    local count=$(echo "$response" | jq 'length')
    echo "✅ [$name] OK ($count 件)"
    ((PASSED++))
    return 0
}

echo "[1] ヘルスチェック"
check_endpoint "health" "/api/v2/health"
check_endpoint "health/db" "/api/v2/health/db"

echo ""
echo "[2] 地磁気データ"
check_json_field "geomag" "/api/v2/propagation/geomag" "date"

echo ""
echo "[3] アクティベーション"
check_array_endpoint "alerts (JA)" "/api/v2/activation/alerts?pat_ref=JA"
check_array_endpoint "spots (JA)" "/api/v2/activation/spots?pat_ref=JA&hours_ago=1"

echo ""
echo "[4] APRS"
# APRSトラックは {"tracks": [...]} 形式で返却される
check_json_field "aprs track" "/api/v2/activation/aprs/track?pat_ref=JA&hours_ago=24" "tracks"

echo ""
echo "[5] 検索API"
check_endpoint "search" "/api/v2/search?min_lat=35&max_lat=36&min_lon=139&max_lon=140" "200" "true"

echo ""
echo "=== 結果 ==="
echo "✅ 成功: $PASSED"
echo "❌ 失敗: $FAILED"

if [ $FAILED -gt 0 ]; then
    echo ""
    echo "⚠️  一部のチェックが失敗しました"
    exit 1
else
    echo ""
    echo "🎉 すべてのチェックが成功しました"
    exit 0
fi
