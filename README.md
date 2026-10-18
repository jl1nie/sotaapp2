# SOTAApp2 - アマチュア無線アワードプログラム管理システム

**MyACTの次世代バックエンドWebサービス**

SOTAApp2は、アマチュア無線のアワードプログラム（SOTA、POTA）を管理するための高性能・高可用性Webサービスです。ヘキサゴナルアーキテクチャを採用し、Rustで実装されています。

## 🏗️ システムアーキテクチャ

### アーキテクチャ概要

本システムは**ヘキサゴナルアーキテクチャ（Clean Architecture）**を採用し、以下の6つのレイヤーで構成されています：

```
┌─────────────────────────────────────────────┐
│                    API                      │ ← RESTエンドポイント・認証・バリデーション
├─────────────────────────────────────────────┤
│                  Service                    │ ← ユースケース・ビジネスロジック
├─────────────────────────────────────────────┤
│                  Domain                     │ ← エンティティ・バリューオブジェクト
├─────────────────────────────────────────────┤
│                  Adapter                    │ ← DB・外部API・APRS・ファイルアクセス
├─────────────────────────────────────────────┤
│                  Registry                   │ ← 依存性注入・モジュール管理
├─────────────────────────────────────────────┤
│                  Common                     │ ← 共通ライブラリ・設定・エラーハンドリング
└─────────────────────────────────────────────┘
```

### 各レイヤーの責務

#### 🌐 API層（`api/`）
- **責務**: RESTエンドポイント、認証・認可、リクエスト/レスポンス処理
- **技術**: Axum、Firebase Auth、CORS
- **主要コンポーネント**:
  - ハンドラー（SOTA、POTA、認証、ヘルスチェック）
  - ミドルウェア（認証、CORS）
  - DTOモデル

#### 🎯 Service層（`service/`）
- **責務**: ユースケース実装、ビジネスルール、トランザクション管理
- **パターン**: Application Service パターン
- **主要サービス**:
  - `UserService`: ユーザー向け機能
  - `AdminService`: 管理者向け機能
  - `AdminPeriodicService`: バッチ処理・定期実行

#### 🏛️ Domain層（`domain/`）
- **責務**: ビジネスエンティティ、バリューオブジェクト、ドメインロジック
- **主要エンティティ**:
  - `Activation`: アクティベーション（Alert/Spot）
  - `SOTAReference`: SOTA山岳データ
  - `POTAReference`: POTA公園データ
  - `AprsLog`: APRS通信ログ

#### 🔌 Adapter層（`adapter/`）
- **責務**: 外部システム連携、データベースアクセス、インフラストラクチャ
- **外部連携**:
  - データベース（SQLite/PostgreSQL）
  - APRS-IS（無線データ）
  - 地磁気データAPI
  - SOTA/POTAオープンデータ

#### 🔧 Registry層（`registry/`）
- **責務**: 依存性注入、モジュール構成管理
- **技術**: Shaku DI フレームワーク
- **設計**: 実行時依存性解決

#### 📚 Common層（`common/`）
- **責務**: 共通ライブラリ、設定管理、エラーハンドリング
- **コンポーネント**: 設定、エラー型、ユーティリティ

## 🚀 主要機能

### SOTA（Summits on the Air）管理
- 山岳データベース管理
- アクティベーション予告（Alert）・運用報告（Spot）
- ログアップロード・進捗管理

### POTA（Parks on the Air）管理
- 公園データベース管理
- アクティベーション・スポット管理

### リアルタイム通信
- APRS-IS連携によるリアルタイム位置情報
- WebSocket対応予定

### データ管理・分析
- 地磁気データ取得・分析
- CSVインポート・エクスポート
- 統計・レポート機能

## 🛠️ 技術スタック

### コアテクノロジー
- **言語**: Rust 2021 Edition
- **Webフレームワーク**: Axum 0.8
- **データベース**: SQLite / PostgreSQL（SQLx）
- **認証**: Firebase Authentication
- **DI**: Shaku
- **非同期**: Tokio
- **ログ**: tracing

### 外部連携
- **APRS**: aprs-message クレート
- **地理情報**: geographiclib-rs、maidenhead
- **スケジュール**: tokio-cron-scheduler

### インフラ
- **コンテナ**: Docker / Docker Compose
- **デプロイ**: Fly.io
- **CI/CD**: GitHub Actions（想定）

## 📋 開発ガイドライン

### レイヤー別メンテナンス方針

| ユースケース | 主要メンテナンス対象 | 変更頻度 | 注意点 |
|-------------|-------------------|----------|--------|
| **新機能追加** | Service → API → Domain | 高 | ドメイン変更は慎重に |
| **外部API変更** | Adapter | 中 | インターフェース維持 |
| **UI/UX改善** | API（ハンドラー・モデル） | 高 | DTOとドメインの分離 |
| **パフォーマンス** | Adapter → Service | 中 | N+1問題、キャッシュ |
| **セキュリティ** | API（認証）→ Common | 低 | 認証・認可ロジック |
| **データ移行** | Adapter（migrations） | 低 | スキーマバージョニング |
| **設定変更** | Common | 中 | 環境変数・設定ファイル |

### 開発フロー推奨事項

1. **ドメインファースト**: 新機能はDomainからスタート
2. **テストファースト**: 各層でユニットテスト実装
3. **依存性の方向**: 内側→外側の依存のみ
4. **インターフェース設計**: traitによる抽象化
5. **エラーハンドリング**: anyhow + thiserrorによる構造化

### コーディング規約

```rust
// ✅ 良い例: トレイトによる抽象化
#[async_trait]
pub trait UserService {
    async fn find_user(&self, id: UserId) -> AppResult<User>;
}

// ✅ 良い例: 型安全性
pub struct SummitCode(String);
impl SummitCode {
    pub fn new(code: String) -> Self {
        // バリデーション
        Self(code)
    }
}

// ❌ 悪い例: 直接的な依存
// Service層からAdapter実装に直接依存
```

## 🚀 開発環境セットアップ

詳細な開発環境構築手順は [docs/development-setup.md](docs/development-setup.md) を参照してください。

### クイックスタート

```bash
git clone https://github.com/jl1nie/sotaapp2.git
cd sotaapp2
cp .env.example .env  # 編集してAPIキー等を設定
makers build && makers migrate run && makers run
```

## 📊 API エンドポイント

本番環境: `https://sotaapp2.fly.dev`

### ヘルスチェック

| エンドポイント | 説明 |
|---------------|------|
| `GET /api/v2/health` | サーバーヘルスチェック |
| `GET /api/v2/health/db` | データベース接続確認 |

### アクティベーション API

| エンドポイント | パラメータ | 説明 |
|---------------|-----------|------|
| `GET /api/v2/activation/alerts` | `pat_ref` (必須) | アラート一覧取得 |
| `GET /api/v2/activation/spots` | `pat_ref` (必須), `hours_ago` | スポット一覧取得 |
| `GET /api/v2/activation/aprs/track` | `pat_ref` (必須), `hours_ago` | APRSトラック取得 |

**パラメータ例:**
- `pat_ref=JA` - 日本のアクティベーション
- `pat_ref=JA,HL` - 日本と韓国
- `hours_ago=24` - 過去24時間

### 地磁気データ API

| エンドポイント | 説明 |
|---------------|------|
| `GET /api/v2/propagation/geomag` | 最新の地磁気指数（A/K指数） |

**レスポンス例:**
```json
{
  "date": "2025-12-26",
  "a_index": 12,
  "k_index": [3.0, 3.33, 2.67]
}
```

### 検索 API

| エンドポイント | パラメータ | 説明 |
|---------------|-----------|------|
| `GET /api/v2/search` | `min_lat`, `max_lat`, `min_lon`, `max_lon` | 範囲内の山岳・公園検索 |

**パラメータ例:**
```
/api/v2/search?min_lat=35&max_lat=36&min_lon=139&max_lon=140
```

### 地図表示 API

| エンドポイント | パラメータ | 説明 |
|---------------|-----------|------|
| `GET /api/v2/map` | `lat` (必須), `lon` (必須), `zoom`, `label`, `tile`, `embed` | 指定座標を中心とした地図をHTMLで表示 |
| `GET /api/v2/map/view` | 同上 | 地図描画に必要な情報をJSONで返す（フロントエンド連携用） |

| パラメータ | 既定値 | 説明 |
|-----------|--------|------|
| `lat` | - | 緯度（-90〜90） |
| `lon` | - | 経度（-180〜180） |
| `zoom` | `15` | ズームレベル（1〜19。タイルの最大ズームに丸められる） |
| `label` | - | マーカーに表示する名称（100文字以内） |
| `tile` | `osm` | タイル種別。`osm` = OpenStreetMap、`gsi` = 地理院タイル |
| `embed` | `false` | `true`でヘッダを省略（iframe埋め込み用） |

**パラメータ例:**
```
/api/v2/map?lat=35.360556&lon=138.727778&zoom=13&label=富士山&tile=gsi
```

ブラウザで開くと、指定座標にマーカーを立てた地図（Leaflet）が表示されます。
マーカーのポップアップには座標とグリッドロケータ（メイデンヘッド）が表示されます。

#### フロントエンドからの利用

`admin/src/lib/api.ts` にクライアントヘルパーを用意しています。

```ts
import { buildMapUrl, getMapView, hasValidCoordinates } from '$lib/api';

// 1. 別タブで開く / iframeのsrcに指定する
const url = buildMapUrl({ lat, lon, label: summitName });
window.open(url, '_blank', 'noopener');

// 2. iframeに埋め込む（ヘッダ非表示）
const embedUrl = buildMapUrl({ lat, lon, label: summitName, embed: true });

// 3. 自前の地図コンポーネントで描画する
const view = await getMapView({ lat, lon, tile: 'gsi' });
// view.tile.urlTemplate / view.tile.attribution / view.tile.maxZoom を
// Leaflet等にそのまま渡せる。view.externalLinks には外部地図サービスへのリンク。
```

`GET /api/v2/map/view` のレスポンス例:

```json
{
  "latitude": 35.360556,
  "longitude": 138.727778,
  "zoom": 15,
  "maidenhead": "PM95ii76",
  "label": "富士山",
  "tile": {
    "kind": "osm",
    "urlTemplate": "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
    "attribution": "&copy; <a href=\"https://www.openstreetmap.org/copyright\">OpenStreetMap</a> contributors",
    "maxZoom": 19
  },
  "mapUrl": "/api/v2/map?lat=35.360556&lon=138.727778&zoom=15&tile=osm&label=%E5%AF%8C%E5%A3%AB%E5%B1%B1",
  "embedUrl": "/api/v2/map?lat=35.360556&lon=138.727778&zoom=15&tile=osm&label=%E5%AF%8C%E5%A3%AB%E5%B1%B1&embed=true",
  "externalLinks": {
    "googleMaps": "https://www.google.com/maps?q=35.360556,138.727778",
    "openStreetMap": "https://www.openstreetmap.org/?mlat=35.360556&mlon=138.727778#map=15/35.360556/138.727778",
    "gsiMaps": "https://maps.gsi.go.jp/#15/35.360556/138.727778/"
  }
}
```

管理コンソール（`admin/`）のSOTA/POTAリファレンス編集画面には、
編集中の座標を地図で開く「🗺 地図で位置を確認」ボタンを追加しています。

## 🔧 設定項目

### 環境変数

| 変数名 | 説明 | デフォルト |
|--------|------|-----------|
| `DATABASE_URL` | データベースURL | `sqlite:sotaapp2.db` |
| `FIREBASE_API_KEY` | Firebase APIキー | - |
| `HOST` | バインドホスト | `0.0.0.0` |
| `PORT` | ポート番号 | `8080` |
| `LOG_LEVEL` | ログレベル | `info` |

詳細は`docker-compose.yaml`参照。

## 🧪 テスト

### ユニットテスト

```bash
# 全テスト実行
makers test

# CI用テスト（fmt-check + clippy-strict + test）
makers ci
```

### Docker E2Eテスト

Dockerイメージに対する包括的なE2Eテストを実行します。

```bash
# Dockerイメージをビルドしてテスト実行
makers e2e

# 既存イメージでテストのみ実行
makers e2e-test jl1nie/sotaapp2:latest

# テストサーバー起動（手動確認用）
makers e2e-server
makers e2e-stop
```

E2Eテストは以下の4フェーズで構成されています：

| フェーズ | テスト内容 |
|---------|-----------|
| BUILD | 実行ファイル・migrations・staticの存在確認 |
| RUNTIME | SSL/TLS接続・ca-certificates・libssl確認 |
| APP | CLIコマンド（help, migrate等）の動作確認 |
| SERVER/API | サーバー起動・ヘルスチェック・主要APIエンドポイント |

詳細は [docs/docker-e2e-test-plan.md](docs/docker-e2e-test-plan.md) を参照。

### CI/CD

GitHub Actionsで以下が自動実行されます：

- **CI Pipeline**: fmt, clippy, ユニットテスト
- **Build Release**: リリースビルド
- **Docker Build & E2E Test**: Dockerビルド・E2Eテスト

## 🚀 デプロイ

詳細は [docs/deployment-manual.md](docs/deployment-manual.md) を参照。

### Fly.ioデプロイ

```bash
# 通常デプロイ（バックアップ付き）
makers deploy

# バックアップなしデプロイ
makers deploy-no-backup

# Fly.io直接デプロイ
makers fly-deploy
```

### データベース管理

```bash
# バックアップ作成
makers fly-backup

# バックアップ一覧
makers fly-db-list

# リストア
makers fly-db-restore /data/backup_YYYYMMDD_HHMMSS.db

# 最適化
makers fly-db-optimize
```

### 本番環境設定
- データベース: SQLite（永続ボリューム）
- メモリ: 256MB
- CPU: shared-1x

## 🤝 コントリビューション

詳しい開発フローは [Repository Guidelines](AGENTS.md) を参照してください。

1. Issueまたは機能提案作成
2. フィーチャーブランチ作成
3. 変更実装（テスト含む）
4. プルリクエスト作成

### プルリクエストチェックリスト
- [ ] テスト追加・更新
- [ ] ドキュメント更新
- [ ] Lintエラー解消
- [ ] APIエンドポイント変更の場合、OpenAPI更新

## 📄 ライセンス

本プロジェクトは[MITライセンス](LICENSE)の下で公開されています。

## 📞 サポート

- 📧 Issue: [GitHub Issues](https://github.com/your-org/sotaapp2/issues)
- 💬 ディスカッション: [GitHub Discussions](https://github.com/your-org/sotaapp2/discussions)

---

**Happy Coding! 73! 🎯📡**

