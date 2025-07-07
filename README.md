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

## 🚀 クイックスタート

### 前提条件
- Rust 1.70+
- Docker & Docker Compose
- SQLite または PostgreSQL

### ローカル開発

```bash
# プロジェクトクローン
git clone https://github.com/your-org/sotaapp2.git
cd sotaapp2

# 依存関係インストール
cargo build

# 設定ファイル準備
cp .env.example .env
# .envファイルを編集

# データベース準備（SQLite）
cargo run --bin migration

# 開発サーバー起動
cargo run --bin app
```

### Docker使用

```bash
# SQLite使用
DOCKER_FILE=Dockerfile.sqlite docker-compose up

# PostgreSQL使用  
DOCKER_FILE=Dockerfile docker-compose up
```

## 📊 API エンドポイント

### SOTA API
```
GET    /api/v2/sota/spots          # スポット一覧
GET    /api/v2/sota/alerts         # アラート一覧
GET    /api/v2/sota/summits        # 山岳一覧
POST   /api/v2/sota/log           # ログアップロード（要認証）
```

### POTA API
```
GET    /api/v2/pota/spots          # スポット一覧
GET    /api/v2/pota/alerts         # アラート一覧
GET    /api/v2/pota/parks          # 公園一覧
```

### 管理API
```
POST   /api/v2/sota/import         # CSVインポート（要管理者権限）
PUT    /api/v2/sota/summits/{code} # 山岳データ更新（要管理者権限）
```

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

```bash
# 全テスト実行
cargo test

# レイヤー別テスト
cargo test --package domain
cargo test --package service  
cargo test --package api
```

## 🚀 デプロイ

### Fly.io
```bash
fly deploy
```

### 本番環境設定
- データベース: PostgreSQL推奨
- メモリ: 512MB以上
- CPU: 1vCPU以上

## 🤝 コントリビューション

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

