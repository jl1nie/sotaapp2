# リファクタリング計画

## 完了済み

### #1 unwrap() の置き換え (API層) ✅
- `api/src/handler/sota.rs`: 日付生成の `.unwrap()` を `?` 演算子に置換
- `api/src/handler/activation.rs`: `serde_json::to_value().unwrap()` を適切なエラーハンドリングに置換

### #2 マルチパートヘルパー関数の作成 (API層) ✅
- `api/src/handler/multipart.rs` を新規作成
- 5つのハンドラで重複していたマルチパート処理を共通化
- 約79行の削減

### #3 設定ファイルの改善 (Common層) ✅
- `common/src/config.rs` でヘルパー関数を追加
- `anyhow::Context` による意味のあるエラーメッセージ
- 必須環境変数: `DATABASE_URL`, `FIREBASE_API_KEY`, `APRSUSER`, `APRSPASSWORD`
- その他はすべてデフォルト値を設定

### #4 エラー型の整理 (Common層) ✅
- `UuidError` と `ConvertToUuidError` の重複を解消（`#[from]`版に統一）
- 認証エラーのHTTPステータスコード修正:
  - `UnauthenticatedError`: 403→401 (UNAUTHORIZED)
  - `UnauthorizedError`: 401→403 (FORBIDDEN)
- コメントアウトされたデッドコード削除（`ValidationError`, `KeyValueStoreError`）

### #5 expect() の置き換え ✅
- `src/bin/app.rs`: IPアドレスパースの `expect()` を `context()` に置換
- Adapter層には `expect()` 使用箇所なし

### #6 認証ミドルウェアの共通化 (API層) ✅
- `auth.rs` に `with_auth()` ヘルパー関数を追加
- `sota.rs`, `pota.rs`, `locator.rs` で重複していた `route_layer(middleware::from_fn_with_state(...))` を共通化
- 各ハンドラから `middleware` インポートを削除

### #8 ログレベル統一 ✅
- `eprintln!` を `tracing::warn!` に置換（auth.rs）
- ログレベルは概ね適切に使い分けられている

### #10 デッドコード削除 / Clippy警告修正 ✅
- `service/src/implement/user_service.rs`: `if x.is_none() { return None }` を `?` に置換
- `adapter/src/database/implement/sqlite/pota_reference.rs`: `is_err()/unwrap()` パターンを `match` に置換

### #15 マジックナンバー排除 ✅
- `api/src/handler/activation.rs`: キャッシュTTLを定数化
  - `CACHE_TTL_SPOTS = 30`
  - `CACHE_TTL_ALERTS = 180`
  - `CACHE_TTL_TRACK = 60`

---

## スキップ / 既に対応済み

### #9 コメントの言語統一
- 日本語と英語が混在しているが、影響範囲が大きいため今回はスキップ

### #11 依存関係の整理
- `cargo-udeps` が未インストールのためスキップ

### #12 型変換の共通化
- `From`/`Into` パターンは既に統一されている

### #13 クエリビルダーパターン統一
- `FindActBuilder`, `FindRefBuilder` 等は既に一貫したAPIを持つ

### #14 リポジトリトレイトの非同期化統一
- `#[async_trait]` は既に統一されている

### #16 Option処理の統一 ✅
- `unwrap_or_default()` と `unwrap_or(0)` は既に適切に使い分けられている
- 変更不要

### #17 HTTPクライアント共有
- `reqwest::get()` と `reqwest::Client::new()` が複数箇所で使用
- 影響範囲が広いため今回はスキップ

### #18 環境変数バリデーション強化 ✅
- `validate_required_env()` 関数を追加
- `AppConfig::new()` の先頭で一括検証
- 不足している必須環境変数を一覧表示

---

## 未着手

### #7 テストの網羅性向上 (全レイヤー)
- 現在はService層の一部のみテスト
- API層のハンドラテスト追加
- Adapter層のリポジトリテスト追加

---

## 優先順位の判断基準

| 優先度 | 基準 |
|--------|------|
| 高 | バグの原因になりうる、保守性に大きく影響 |
| 中 | コードの可読性向上、将来の拡張性 |
| 低 | 美観、一貫性のみに関わる |
