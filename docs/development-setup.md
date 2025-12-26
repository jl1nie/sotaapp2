# 開発環境セットアップガイド

SOTAApp2の開発環境を構築するための詳細な手順です。

## 必要なツール

### 必須

| ツール | バージョン | インストール方法 |
|--------|-----------|-----------------|
| Rust | 1.83+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| cargo-make | 最新 | `cargo install cargo-make` |
| cargo-nextest | 最新 | `cargo install cargo-nextest` |
| SQLite | 3.x | OS標準パッケージマネージャ |

### オプション（Docker使用時）

| ツール | 用途 |
|--------|------|
| Docker | コンテナビルド・実行 |
| Docker Compose | マルチコンテナ管理 |

### オプション（本番デプロイ時）

| ツール | インストール方法 |
|--------|-----------------|
| Fly CLI | `curl -L https://fly.io/install.sh \| sh` |
| GitHub CLI | `brew install gh` または [公式サイト](https://cli.github.com/) |

## セットアップ手順

### 1. リポジトリのクローン

```bash
git clone https://github.com/jl1nie/sotaapp2.git
cd sotaapp2
```

### 2. 環境変数の設定

```bash
cp .env.example .env
```

`.env`ファイルを編集して、以下の項目を設定してください：

#### 必須設定

| 変数 | 説明 | 取得方法 |
|------|------|----------|
| `FIREBASE_API_KEY` | Firebase認証用APIキー | [Firebase Console](https://console.firebase.google.com/) → プロジェクト設定 → ウェブAPIキー |
| `APRSUSER` | APRSコールサイン | 自分のコールサイン + SSID（例: `JA1XXX-10`） |
| `APRSPASSWORD` | APRSパスコード | [Passcode Generator](https://apps.magicbug.co.uk/passcode/) で生成 |

#### オプション設定

| 変数 | 説明 | デフォルト |
|------|------|-----------|
| `DATABASE_URL` | SQLiteデータベースパス | `sqlite:./sotaapp2.db` |
| `LOG_LEVEL` | ログレベル | `info` |
| `PORT` | サーバーポート | `8080` |

### 3. ビルド

```bash
makers build
```

### 4. データベースマイグレーション

```bash
makers migrate run
```

### 5. 開発サーバー起動

```bash
makers run
```

サーバーが起動したら、以下のURLで動作確認できます：

- ヘルスチェック: http://localhost:8080/api/v2/health
- 地磁気データ: http://localhost:8080/api/v2/propagation/geomag

## 開発コマンド一覧

### ビルド・実行

| コマンド | 説明 |
|----------|------|
| `makers build` | デバッグビルド |
| `makers build-release` | リリースビルド |
| `makers run` | 開発サーバー起動 |
| `makers watch` | ファイル変更監視（fmt, clippy, test自動実行） |

### テスト

| コマンド | 説明 |
|----------|------|
| `makers test` | 全テスト実行 |
| `makers ci` | CI相当のチェック（fmt-check + clippy-strict + test） |
| `makers e2e` | DockerビルドとE2Eテスト |

### コード品質

| コマンド | 説明 |
|----------|------|
| `makers fmt` | コードフォーマット |
| `makers fmt-check` | フォーマットチェック |
| `makers clippy` | Lintチェック |
| `makers clippy-strict` | 厳格Lintチェック（警告=エラー） |

### データベース

| コマンド | 説明 |
|----------|------|
| `makers migrate run` | マイグレーション実行 |
| `makers migrate add <name>` | 新規マイグレーション追加 |

### Docker

| コマンド | 説明 |
|----------|------|
| `makers compose-build-app` | Dockerイメージビルド |
| `makers run-in-docker` | Dockerコンテナ起動 |
| `makers compose-down` | コンテナ停止 |
| `makers e2e-server` | E2Eテストサーバー起動 |
| `makers e2e-stop` | E2Eテストサーバー停止 |

## テスト用環境変数

テスト実行時は`.env.test`が使用されます：

```bash
cp .env.example .env.test
# 必要に応じて編集
```

## トラブルシューティング

### ビルドエラー: SQLiteが見つからない

```bash
# Ubuntu/Debian
sudo apt-get install libsqlite3-dev

# macOS
brew install sqlite
```

### APRS接続エラー

- `APRSUSER`が正しいコールサイン形式か確認
- `APRSPASSWORD`が正しいパスコードか確認
- ネットワーク接続を確認

### マイグレーションエラー

```bash
# データベースファイルを削除して再作成
rm sotaapp2.db
makers migrate run
```

### ポート8080が使用中

```bash
# .envでポートを変更
PORT="18080"
```

## IDE設定

### VS Code推奨拡張

- rust-analyzer
- Even Better TOML
- Error Lens

### .vscode/settings.json

```json
{
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.cargo.features": "all"
}
```

## 次のステップ

- [デプロイメントマニュアル](deployment-manual.md) - 本番環境へのデプロイ
- [E2Eテスト計画](docker-e2e-test-plan.md) - Docker E2Eテストの詳細
- [CLAUDE.md](../CLAUDE.md) - AI支援開発ガイドライン
