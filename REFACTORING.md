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

---

## 未着手

### #5 expect() の置き換え (Adapter層)
- `adapter/src/database/` 内の `expect()` を `?` または `anyhow::Context` に置換
- パニックの可能性を排除

### #6 認証ミドルウェアの共通化 (API層)
- 現在各ハンドラで `route_layer(middleware::from_fn_with_state(...))` を重複記述
- ルーター構築時に一括適用する仕組みに変更

### #7 テストの網羅性向上 (全レイヤー)
- 現在はService層の一部のみテスト
- API層のハンドラテスト追加
- Adapter層のリポジトリテスト追加

### #8 ログレベル統一 (全レイヤー)
- `tracing::error!` の使用箇所を確認
- 適切なログレベル（debug, info, warn, error）の使い分け

### #9 コメントの言語統一
- 日本語コメントと英語コメントが混在
- プロジェクト方針として統一

### #10 デッドコード削除
- コメントアウトされたコード
- 未使用のインポート
- 未使用の関数

### #11 依存関係の整理 (Cargo.toml)
- 各crateの依存関係を精査
- 未使用依存の削除
- バージョン統一

### #12 型変換の共通化 (Domain/Service層)
- `From`/`Into` トレイトの実装パターン統一
- 変換ロジックの重複排除

### #13 クエリビルダーパターン統一 (Domain層)
- `FindActBuilder`, `FindRefBuilder`, `FindLogBuilder` のAPI統一
- メソッドチェーンの一貫性

### #14 リポジトリトレイトの非同期化統一 (Domain層)
- 一部同期的な実装が残っている可能性
- `async_trait` の一貫した使用

### #15 マジックナンバー排除
- コード内のハードコードされた数値を定数化
- 例: キャッシュTTL、デフォルト期間など

### #16 Option処理の統一
- `unwrap_or_default()` vs `unwrap_or(0)` の使い分け
- `if let Some` vs `map`/`and_then` の使い分け

### #17 HTTPクライアント共有
- 各所で `reqwest::Client` を個別生成
- 接続プール共有による効率化

### #18 環境変数バリデーション強化
- 起動時に必須環境変数を一括検証
- 不足時は早期に明確なエラー

---

## 優先順位の判断基準

| 優先度 | 基準 |
|--------|------|
| 高 | バグの原因になりうる、保守性に大きく影響 |
| 中 | コードの可読性向上、将来の拡張性 |
| 低 | 美観、一貫性のみに関わる |

## 次のステップ推奨

**#5 expect() の置き換え** を推奨する理由:
1. パニックの可能性を排除し、安全性向上
2. Adapter層に限定されるため影響範囲が明確
3. #1-4で確立したパターンを適用可能

**#6 認証ミドルウェアの共通化** も検討:
- 各ルーターで `route_layer(middleware::from_fn_with_state(...))` が重複
- 一括適用でコード削減と保守性向上
