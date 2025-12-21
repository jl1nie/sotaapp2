# Docker E2E テスト計画

## 概要

本ドキュメントは、sotaapp2のDockerイメージに対するEnd-to-End（E2E）テストの設計と実装計画を定義する。

### 背景

2024-12-21のデプロイで以下の問題が発生した：
- `ca-certificates`パッケージの欠落によりHTTPS通信が失敗
- `migrations/`ディレクトリの欠落によりアプリ起動失敗
- CIでは検出できず、本番デプロイ後に発覚

これらは体系的なE2Eテストがあれば防げた問題である。

## テスト観点

### 1. ビルド成果物の検証

Dockerイメージ内に必要なファイルが正しく配置されているかを検証する。

| テストID | テスト内容 | 検証方法 | 優先度 |
|----------|-----------|----------|--------|
| BUILD-001 | 実行ファイルの存在 | `/app/target/release/app` の存在確認 | 高 |
| BUILD-002 | 実行ファイルの権限 | 実行可能フラグの確認 | 高 |
| BUILD-003 | migrationsディレクトリ | `/app/migrations/` の存在と中身 | 高 |
| BUILD-004 | staticディレクトリ | `/app/static/` の存在と中身 | 高 |
| BUILD-005 | index.htmlの存在 | `/app/static/index.html` の存在 | 中 |

### 2. ランタイム依存関係

アプリケーション実行に必要なシステムライブラリ・証明書の検証。

| テストID | テスト内容 | 検証方法 | 優先度 |
|----------|-----------|----------|--------|
| RUNTIME-001 | SSL/TLS証明書 | HTTPS接続テスト（NOAA等） | 高 |
| RUNTIME-002 | libssl | `ldd`でリンク確認 | 中 |
| RUNTIME-003 | ca-certificatesパス | `/etc/ssl/certs/` の存在 | 高 |

### 3. アプリケーション起動テスト

アプリケーションの各サブコマンドが正常に動作するかを検証。

| テストID | テスト内容 | 検証方法 | 優先度 |
|----------|-----------|----------|--------|
| APP-001 | --help表示 | 終了コード0、出力確認 | 高 |
| APP-002 | 環境変数不足エラー | 適切なエラーメッセージ | 高 |
| APP-003 | db migrate | 新規DBでマイグレーション成功 | 高 |
| APP-004 | db backup | バックアップ作成成功 | 中 |
| APP-005 | db vacuum | 実行成功 | 低 |

### 4. サーバー起動・ヘルスチェック

サーバーが正常に起動し、ヘルスチェックに応答するかを検証。

| テストID | テスト内容 | 検証方法 | 優先度 |
|----------|-----------|----------|--------|
| SERVER-001 | サーバー起動 | 指定ポートでリッスン開始 | 高 |
| SERVER-002 | /health エンドポイント | 200 OK応答 | 高 |
| SERVER-003 | 起動時間 | 60秒以内に応答可能 | 中 |
| SERVER-004 | Graceful shutdown | SIGTERMで正常終了 | 中 |

### 5. APIスモークテスト

主要APIエンドポイントが応答するかを検証（詳細なロジックテストではない）。

| テストID | テスト内容 | 検証方法 | 優先度 |
|----------|-----------|----------|--------|
| API-001 | GET /api/v2/propagation/geomag | 200応答、JSONフォーマット | 高 |
| API-002 | GET /api/v2/spots | 200応答 | 高 |
| API-003 | GET /api/v2/alerts | 200応答 | 高 |
| API-004 | GET /api/v2/activation/aprs/track | 200応答 | 中 |
| API-005 | 静的ファイル配信 | /index.html 200応答 | 中 |

## 実装方針

### ファイル構成

```
scripts/
  docker-e2e-test.sh        # メインテストスクリプト
  e2e/
    test-build.sh           # BUILD-* テスト
    test-runtime.sh         # RUNTIME-* テスト
    test-app.sh             # APP-* テスト
    test-server.sh          # SERVER-*, API-* テスト
    lib.sh                  # 共通関数

docker-compose.test.yml     # テスト用compose設定
```

### テスト実行フロー

```
1. Dockerイメージのビルド
2. BUILD-* テスト実行（イメージ内ファイル確認）
3. RUNTIME-* テスト実行（依存関係確認）
4. APP-* テスト実行（サブコマンド確認）
5. docker-compose.test.yml でサーバー起動
6. SERVER-* テスト実行（ヘルスチェック）
7. API-* テスト実行（エンドポイント確認）
8. クリーンアップ
```

### CI統合

```yaml
# .github/workflows/ci.yml

docker:
  name: Docker Build & E2E Test
  runs-on: ubuntu-latest
  needs: ci
  steps:
    - uses: actions/checkout@v4

    - name: Build Docker image
      run: docker build -t sotaapp2:test .

    - name: Run E2E tests
      run: ./scripts/docker-e2e-test.sh sotaapp2:test
```

### テストスクリプト設計

#### docker-e2e-test.sh（メインスクリプト）

```bash
#!/bin/bash
set -euo pipefail

IMAGE="${1:-sotaapp2:test}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

source "$SCRIPT_DIR/e2e/lib.sh"

log_header "Docker E2E Test Suite"
log_info "Testing image: $IMAGE"

# Phase 1: Build artifacts
log_header "Phase 1: Build Artifacts"
"$SCRIPT_DIR/e2e/test-build.sh" "$IMAGE"

# Phase 2: Runtime dependencies
log_header "Phase 2: Runtime Dependencies"
"$SCRIPT_DIR/e2e/test-runtime.sh" "$IMAGE"

# Phase 3: Application commands
log_header "Phase 3: Application Commands"
"$SCRIPT_DIR/e2e/test-app.sh" "$IMAGE"

# Phase 4: Server & API (requires docker-compose)
log_header "Phase 4: Server & API"
"$SCRIPT_DIR/e2e/test-server.sh" "$IMAGE"

log_header "All E2E tests passed!"
```

#### lib.sh（共通関数）

```bash
#!/bin/bash

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }
log_header() { echo -e "\n${GREEN}=== $1 ===${NC}"; }

assert_success() {
  local name="$1"
  shift
  if "$@"; then
    log_info "✓ $name"
  else
    log_error "✗ $name"
    exit 1
  fi
}

assert_contains() {
  local name="$1"
  local pattern="$2"
  local output="$3"
  if echo "$output" | grep -q "$pattern"; then
    log_info "✓ $name"
  else
    log_error "✗ $name (pattern '$pattern' not found)"
    exit 1
  fi
}

docker_run() {
  docker run --rm "$@"
}

docker_run_shell() {
  local image="$1"
  shift
  docker run --rm --entrypoint="" "$image" sh -c "$*"
}
```

### docker-compose.test.yml

```yaml
version: '3.8'

services:
  app:
    image: ${TEST_IMAGE:-sotaapp2:test}
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: "sqlite:/tmp/test.db?mode=rwc"
      FIREBASE_API_KEY: "test-key"
      APRSUSER: "testuser"
      APRSPASSWORD: "testpass"
      MIGRATION_PATH: "/app/migrations"
      HOST: "0.0.0.0"
      PORT: "8080"
      GEOMAG_ENDPOINT: "https://services.swpc.noaa.gov/text/daily-geomagnetic-indices.txt"
      GEOMAG_SCHEDULE: "0 0 */3 * * *"
      SOTA_SPOT_ENDPOINT: "https://api2.sota.org.uk/api/spots/20?"
      SOTA_ALERT_ENDPOINT: "https://api2.sota.org.uk/api/alerts"
      POTA_SPOT_ENDPOINT: "https://api.pota.app/spot/activator"
      POTA_ALERT_ENDPOINT: "https://api.pota.app/activation"
      SPOT_INTERVAL: "60"
      ALERT_INTERVAL: "300"
      SPOT_EXPIRE: "3600"
      ALERT_EXPIRE: "86400"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 5s
      timeout: 5s
      retries: 10
      start_period: 60s
```

## 実装優先順位

### Phase 1（最小限 - 即座に実装）
- BUILD-001, BUILD-003, BUILD-004
- RUNTIME-001, RUNTIME-003
- APP-001, APP-002, APP-003

### Phase 2（中程度 - 次回実装）
- SERVER-001, SERVER-002
- API-001

### Phase 3（包括的 - 将来実装）
- 残りのすべてのテスト
- パフォーマンステスト
- 負荷テスト

## 今後の課題

1. **テスト環境の分離** - 本番APIへの依存を減らすモック検討
2. **テスト実行時間** - 並列実行の検討
3. **レポート出力** - JUnit XML形式での結果出力
4. **ローカル実行** - 開発者がローカルで簡単に実行できる仕組み

## 変更履歴

| 日付 | バージョン | 内容 |
|------|-----------|------|
| 2024-12-21 | 1.0 | 初版作成 |
