# デプロイメントマニュアル

## 概要

本ドキュメントはsotaapp2のテスト・デプロイ手順を説明します。

## 前提条件

### 必要なツール

```bash
# Rust開発環境
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# cargo-make（タスクランナー）
cargo install cargo-make

# Fly.io CLI
curl -L https://fly.io/install.sh | sh
fly auth login

# Docker
# https://docs.docker.com/get-docker/
```

### 環境設定

```bash
# 開発用環境変数
cp .env.example .env
# .envを編集

# テスト用環境変数
cp .env.test.example .env.test
# .env.testを編集
```

## テスト

### ローカルテスト

```bash
# コードフォーマットチェック
makers fmt-check

# Lintチェック
makers clippy

# ユニットテスト
makers test

# CI全体（fmt-check + clippy-strict + test）
makers ci
```

### Docker E2Eテスト

```bash
# Dockerイメージビルド + E2Eテスト実行
makers e2e

# 既存イメージでE2Eテストのみ
makers e2e-test sotaapp2:test
makers e2e-test jl1nie/sotaapp2:latest

# E2Eテストサーバー起動（手動確認用）
makers e2e-server
# http://localhost:18080 でアクセス可能
makers e2e-stop
```

#### E2Eテスト項目

| フェーズ | テストID | 内容 |
|---------|----------|------|
| BUILD | BUILD-001〜005 | 実行ファイル、migrations、static確認 |
| RUNTIME | RUNTIME-001〜003 | SSL/TLS接続、ca-certificates確認 |
| APP | APP-001〜004 | CLIコマンド動作確認 |
| SERVER | SERVER-001〜004 | サーバー起動、ヘルスチェック |
| API | API-001〜005 | 主要APIエンドポイント確認 |

詳細: [docker-e2e-test-plan.md](docker-e2e-test-plan.md)

## デプロイ

### 通常デプロイ（推奨）

タグをプッシュしてCI/CD経由でデプロイします。

```bash
# 1. 全テスト通過を確認
makers ci

# 2. E2Eテスト通過を確認
makers e2e

# 3. タグ作成・プッシュ
git tag v0.1.x
git push origin v0.1.x

# 4. GitHub ActionsでCI・デプロイが自動実行される
# https://github.com/jl1nie/sotaapp2/actions
```

### 手動デプロイ

```bash
# バックアップ付きデプロイ（推奨）
makers deploy

# バックアップなしデプロイ
makers deploy-no-backup

# Fly.io直接デプロイ
makers fly-deploy
```

### 緊急リカバリーデプロイ

サービス障害時に過去のイメージでデプロイします。

```bash
# 利用可能なイメージ確認
docker images | grep sotaapp2

# 特定イメージでデプロイ
fly deploy --image jl1nie/sotaapp2:recovery3

# または以前のリリースタグ
fly deploy --image jl1nie/sotaapp2:v0.1.2
```

## データベース管理

### バックアップ

```bash
# 手動バックアップ
makers fly-backup

# バックアップ一覧確認
makers fly-db-list
```

### リストア

```bash
# リストア実行
makers fly-db-restore /data/backup_YYYYMMDD_HHMMSS.db

# アプリ再起動
fly apps restart
```

### 最適化

```bash
# VACUUM + ANALYZE実行
makers fly-db-optimize

# マイグレーション実行
makers fly-db-migrate
```

## 監視

### ログ確認

```bash
# リアルタイムログ
makers fly-logs

# 過去ログ（直近100行）
fly logs --no-tail | tail -100
```

### ステータス確認

```bash
# アプリステータス
makers fly-status

# マシン一覧
fly machine list

# ヘルスチェック
curl https://sotaapp2.fly.dev/health
```

### SSH接続

```bash
makers fly-ssh
```

## トラブルシューティング

### サービスが起動しない

1. ログ確認
   ```bash
   fly logs --no-tail | tail -50
   ```

2. 環境変数確認
   ```bash
   fly secrets list
   ```

3. リカバリーイメージでデプロイ
   ```bash
   fly deploy --image jl1nie/sotaapp2:recovery3
   ```

### データベースエラー

1. マイグレーション実行
   ```bash
   makers fly-db-migrate
   ```

2. データベース最適化
   ```bash
   makers fly-db-optimize
   ```

3. バックアップからリストア
   ```bash
   makers fly-db-list
   makers fly-db-restore /data/backup_YYYYMMDD_HHMMSS.db
   ```

### SSL/TLS接続エラー

Dockerイメージに`ca-certificates`がインストールされているか確認:

```bash
makers e2e-test sotaapp2:test
# RUNTIME-001: SSL/TLS connectivity が成功すること
```

## リリースチェックリスト

### リリース前

- [ ] `makers ci` 通過
- [ ] `makers e2e` 通過
- [ ] 変更内容のレビュー完了
- [ ] 必要に応じてドキュメント更新

### リリース実行

- [ ] `git tag vX.Y.Z && git push origin vX.Y.Z`
- [ ] GitHub Actions CI通過確認
- [ ] GitHub Actions Deploy成功確認

### リリース後

- [ ] `curl https://sotaapp2.fly.dev/health` で200応答確認
- [ ] 主要機能の動作確認
- [ ] ログにエラーがないことを確認

## 環境情報

### 本番環境（Fly.io）

| 項目 | 値 |
|------|-----|
| App名 | sotaapp2 |
| リージョン | nrt (東京) |
| マシン | shared-cpu-1x, 256MB |
| ボリューム | /data (1GB) |
| データベース | SQLite |

### Dockerイメージ

| タグ | 用途 |
|-----|------|
| latest | 最新ビルド |
| vX.Y.Z | リリースバージョン |
| recovery3 | リカバリー用（ca-certificates対応） |

## 変更履歴

| 日付 | バージョン | 内容 |
|------|-----------|------|
| 2024-12-21 | 1.0 | 初版作成 |
