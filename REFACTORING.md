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

### #16 Option処理の統一 ✅
- `unwrap_or_default()` と `unwrap_or(0)` は既に適切に使い分けられている
- 変更不要

### #17 HTTPクライアント共有 ✅
- `common/src/http.rs` を新規作成
- `OnceLock` による遅延初期化で共有クライアントを実装
- 30秒のデフォルトタイムアウト
- 5箇所の `reqwest::get()` / `reqwest::Client::new()` を置換

### #18 環境変数バリデーション強化 ✅
- `validate_required_env()` 関数を追加
- `AppConfig::new()` の先頭で一括検証
- 不足している必須環境変数を一覧表示

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

---

## 新規リファクタリング項目（コードレビュー結果）

### 【セキュリティ】

#### #19 PostGIS SQLインジェクション対策 ✅
**ファイル**: `adapter/src/database/implement/postgis/querybuilder.rs`
**問題**: 文字列フォーマットによるSQL生成がSQLインジェクションに脆弱
**対策**:
- SQLx の `QueryBuilder<Postgres>` を使用したパラメタライズドクエリに書き換え
- 新規API: `build_sota_ref_query()`, `build_pota_ref_query()`, `build_wwff_ref_query()`, `build_activation_query()`
- 旧API: `#[deprecated]` 属性を付与（後方互換性維持）

---

### 【コード重複】

#### #20 SQLite/PostGIS実装の共通化 ⏭️ SKIP
**理由**: fly.ioのメモリ制限によりPostgreSQLは使用していない
- SQLite版のみをメンテナンス対象とする
- PostGIS版は参考実装として保持

---

### 【ファイル分割 - 高優先】

#### #21 user_service.rs の分割 🟠 HIGH
**ファイル**: `service/src/implement/user_service.rs` (1117行)
**問題**: 単一ファイルに過剰な責務
**対策**:
```
service/src/implement/
  ├── user_service.rs (300行に削減)
  ├── user_activation_service.rs (新規, ~400行)
  ├── award_calculator.rs (新規, ~300行)
  └── activation_grouping.rs (新規, ~100行)
```
**優先度**: 高
**複雑度**: 高

#### #22 ハンドラ関数の重複排除 🟡 MEDIUM
**ファイル**: `api/src/handler/activation.rs` (27-193行)
**問題**: `show_spots`, `show_alerts` に類似パターン4回繰り返し
**対策**:
- ジェネリックハンドラビルダー作成
- 共通キャッシュロジック抽出
- パラメータ抽出のユーティリティ化

**優先度**: 中
**複雑度**: 中
**削減見込み**: ~80行

---

### 【エラーハンドリング - 高優先】

#### #23 unwrap() の一掃 🟠 HIGH
**問題**: 96箇所の `unwrap()` 使用
**重要箇所**:
- `service/src/implement/user_service.rs:210`: `LogId::from_str().unwrap_or()`
- `service/src/implement/user_service.rs:298-299`: ハードコードされた日付
- `service/src/implement/aprs_service.rs:24`: `Regex::new().unwrap()`

**対策**:
- 全 `unwrap()` を `?`, `.map_err()`, `.unwrap_or_default()` に置換
- クリティカルパスでパニックフリー保証

**優先度**: 高
**複雑度**: 中

#### #24 エラーコンテキストの統一 🟡 MEDIUM
**問題**: エラー発生位置情報が不統一
**対策**:
- エラーラッピングマクロ作成
- `#[track_caller]` 属性活用
- エラーハンドリングガイドライン策定

**優先度**: 中
**複雑度**: 低

---

### 【パフォーマンス - 高優先】

#### #25 POTA統計のN+1クエリ問題 🟠 HIGH
**ファイル**: `adapter/src/database/implement/sqlite/pota_reference.rs:280-310`
**問題**: ループ内でクエリ実行（10,000エントリで10,000クエリ）
```rust
for l in logs {
    let r = sqlx::query!(...).fetch_one(...) // ループ内クエリ!
}
```
**対策**:
- SQL JOINで単一クエリ化
- ウィンドウ関数で集計
- 結果のキャッシュ（TTL付き）

**優先度**: 高
**複雑度**: 中

#### #26 Regexランタイムコンパイル 🟡 MEDIUM
**ファイル**: `service/src/implement/aprs_service.rs:24`
**問題**: 呼び出しごとに正規表現をコンパイル
**対策**:
- `lazy_static` または `once_cell` でキャッシュ
- 起動時にパターン検証

**優先度**: 中
**複雑度**: 低

#### #27 過剰なclone()削減 🟡 MEDIUM
**問題**: 115箇所の `clone()` 呼び出し
**対策**:
- 参照で済む箇所を特定
- `Arc<T>` で共有所有権
- 小型の型に `Copy` 実装

**優先度**: 中
**複雑度**: 中
**効果見込み**: 5-15%のパフォーマンス改善

---

### 【テスト - 高優先】

#### #7 テストの網羅性向上 🟠 HIGH
**現状**: 132ファイル中1ファイルのみテストあり（user_service.rs）
**カバレッジ**:
- Database層: 0%
- API handlers: 0%
- Service層: 5%（award logicのみ）
- Configuration: 0%

**対策**:
- `tokio::test` インフラ整備
- 統合テストフレームワーク
- データベースフィクスチャ/モック
- 70%以上のカバレッジ目標

**優先度**: 高
**複雑度**: 高
**工数見込み**: 40時間以上

---

### 【アーキテクチャ - 中優先】

#### #28 クエリビルダー抽象化 🟡 MEDIUM
**問題**: SQLiteとPostGISで異なるクエリビルダーパターン
**対策**:
- `SqlQueryBuilder` トレイト作成
- データベースタイプごとに実装
- 条件ビルダーをコンポーザブルに

**優先度**: 中
**複雑度**: 高

#### #29 CSVモデル変換の共通化 🟡 MEDIUM
**問題**: CSVデシリアライズと変換が散在
**対策**:
- `CsvModel<T>` トレイト作成
- 共通変換をユーティリティに抽出
- 統一インポートパイプライン

**優先度**: 中
**複雑度**: 中

#### #30 Groupingストラテジー抽象化 🟢 LOW
**ファイル**: `service/src/implement/user_service.rs:52-72`
**問題**: `get_alert_group()` と `get_spot_group()` がほぼ同一
**対策**:
- `Grouper<T>` トレイト作成
- Alert/Spotに実装
- ジェネリックハンドラ関数

**優先度**: 低
**複雑度**: 低

---

### 【設定・保守性 - 中優先】

#### #31 ハードコード日付の設定化 🟡 MEDIUM
**問題**: 日付がコードに散在
- `service/src/implement/user_service.rs:298-299`: アワード期間
- `api/src/handler/sota.rs:106-111`: 日付範囲

**対策**:
- 日付設定の集中管理
- 環境変数またはDB設定化
- `DateRange` 型作成

**優先度**: 中
**複雑度**: 低

#### #32 モジュールドキュメント追加 🟢 LOW
**問題**: ほとんどのファイルにモジュールドキュメントなし
**対策**:
- 全publicモジュールに `//!` ドキュメント
- アーキテクチャドキュメント作成
- 設定要件ドキュメント

**優先度**: 低
**複雑度**: 低

---

## 優先順位サマリー

| カテゴリ | 項目数 | 優先度 | 複雑度 | 工数目安 |
|----------|--------|--------|--------|----------|
| セキュリティ | 1 | ✅完了 | - | - |
| ファイル分割 | 2 | 高 | 高 | 20-30h |
| エラーハンドリング | 2 | 高 | 中 | 20-25h |
| パフォーマンス | 3 | 高 | 中 | 15-20h |
| テスト | 1 | 高 | 高 | 40h+ |
| アーキテクチャ | 3 | 中 | 中-高 | 30-40h |
| 設定・保守性 | 2 | 低-中 | 低 | 10-15h |

## 推奨実施順序

1. ~~**#19 SQLインジェクション対策**~~ ✅ 完了
2. **#7 テストインフラ整備** - 大規模リファクタリング前に
3. **#23 unwrap()一掃** - 安定性向上
4. **#25 N+1クエリ修正** - パフォーマンス改善
5. **#21 user_service分割** - 可読性向上
6. **#22 ハンドラ関数の重複排除** - 保守性向上
7. 残りは優先度順に実施
