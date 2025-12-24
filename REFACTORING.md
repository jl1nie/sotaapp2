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

#### #21 user_service.rs の分割 ✅
**ファイル**: `service/src/implement/user_service.rs` (1131行→503行)
**問題**: 単一ファイルに過剰な責務
**対策**:
```
service/src/implement/
  ├── user_service.rs (503行に削減)
  └── award_calculator.rs (新規, 628行) - アワード判定ロジック + 16テスト
```
- `detect_log_type()`, `judge_award_with_mode()`, `evaluate_summit_activation()` を抽出
- テストも新モジュールに移動
- 削減: 628行（55%削減）

#### #22 ハンドラ関数の重複排除 ✅
**ファイル**: `api/src/handler/activation.rs`
**対策**:
- `apply_common_filters()` ヘルパー関数を追加
- グルーピング、時間フィルタ、パターンフィルタ、ログIDフィルタを共通化
- `show_spots`, `show_alerts` の重複コードを削減（約40行削減）

---

### 【エラーハンドリング - 高優先】

#### #23 unwrap() の一掃 ✅
**対策**:
- aprs_service.rs: unwrap() を let-else パターン、is_some_and() に置換
- user_service.rs: 日付のunwrap()を.single().unwrap_or_else()に変更
- award.rs: is_activation()/is_chase()をis_some_and()で安全化

#### #24 エラーコンテキストの統一 ✅
**問題**: エラー発生位置情報が不統一
**対策完了**:
- Phase 1: `db_error()`, `tx_error()`, `row_not_found()` ヘルパー関数をerrorレベルログ出力に更新
- Phase 2: PostGIS全ファイル（5ファイル、59箇所）を新ヘルパー関数に移行
  - activation.rs: 14箇所
  - aprslog.rs: 8箇所
  - locator.rs: 4箇所
  - pota_reference.rs: 20箇所
  - sota_reference.rs: 13箇所
- SQLite版は既に移行済み（84箇所）

**エラー発生時のログ出力例**:
```
DB error in 'insert sota_references postgis': RowNotFound
Transaction error in 'commit create_reference sota postgis': ...
```

---

### 【パフォーマンス - 高優先】

#### #25 POTA統計のN+1クエリ問題 ✅
**ファイル**: `adapter/src/database/implement/sqlite/pota_reference.rs`
**対策**:
- log_stat()内のN+1クエリをJOINクエリに書き換え
- O(n)クエリをO(1)に削減

#### #26 Regexランタイムコンパイル ✅
**ファイル**: `service/src/implement/aprs_service.rs`
**対策**:
- `OnceLock` を使用して正規表現をキャッシュ
- `get_cached_regex()` 関数で共通パターンを事前コンパイル

#### #27 過剰なclone()削減 ✅
**問題**: 117箇所の `clone()` 呼び出し
**対策**:
- SQLite querybuilder: `.clone().unwrap()` → 参照+`as_str()` に変更
- aprs_service: entry API最適化、`as_deref()`使用、重複clone削減
- 結果: 117 → 110 (7箇所削減、6%削減)
- 残りはHashMapキーやフィールド所有権で必要なclone

---

### 【テスト - 高優先】

#### #7 テストの網羅性向上 ✅
**現状**: 81テスト（Phase 1-3 完了）
**カバレッジ**:
- Database層: 7テスト (SQLite CRUD)
- API handlers: 2テスト (health endpoint)
- Service層: 37テスト (award_calculator 27件 + user_service 5件 + admin_service 5件)
- Domain/Model: 23テスト (AprsState 13件 + リポジトリモック 10件)
- Common/Utils: 12テスト (既存)

---

## テスト計画 ✅ 完了

### Phase 1: ユニットテスト（Pure Function） ✅
外部依存なしでテスト可能な関数から着手

| モジュール | ファイル | テスト対象 | 状態 |
|-----------|---------|-----------|--------|
| common | utils.rs | `parse_date_flexible()`, `calculate_distance()`, `maidenhead()` | ✅ |
| service/model | award.rs | `AwardPeriod::contains()`, `SotaLogEntry` パースメソッド | ✅ |
| service | award_calculator.rs | `detect_log_type()`, `judge_award_with_mode()` | ✅ 27件 |

### Phase 2: モック依存テスト ✅
リポジトリをモック化してサービス層をテスト

| モジュール | ファイル | テスト対象 | 状態 |
|-----------|---------|-----------|------|
| domain | repository/sota.rs | MockSotaRepository | ✅ 4件 |
| domain | repository/activation.rs | MockActivationRepositry | ✅ 6件 |
| domain | model/aprslog.rs | AprsState メソッド | ✅ 13件 |
| service | user_service.rs | get_alert_group, get_spot_group, CSV判定 | ✅ 7件 |
| service | admin_service.rs | is_valid_summit | ✅ 5件 |

### Phase 3: 統合テスト ✅
実際のSQLiteデータベースを使用

| 対象 | テスト内容 | 状態 |
|-----|----------|------|
| adapter/sqlite | CRUD操作 (create, find, update, upsert, delete, pagination) | ✅ 7件 |
| api/handler | health エンドポイント | ✅ 2件 |

### テストインフラ要件 ✅
1. **テスト用クレート追加**: ✅
   - `mockall` (モック生成) - domain, Cargo.toml
   - `axum-test` (APIテスト) - api/Cargo.toml
   - `tempfile` (一時ファイル) - adapter/Cargo.toml, Cargo.toml
   - `tower` (テスト用) - api/Cargo.toml

2. **テストヘルパー**: ✅
   - `setup_test_db()` - adapter/sqlite/sota_reference.rs
   - `make_test_reference()` - 各テストモジュール
   - `make_test_alert()`, `make_test_spot()` - repository tests

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

#### #29 CSVモデル変換の共通化 ✅
**問題**: CSVデシリアライズと変換が散在、unwrap()残存
**対策**:
- `parse_date_flexible()` ヘルパー関数を `common/utils.rs` に追加
- pota.rs: unwrap()を除去、to_log()をOption返却に変更
- sota.rs: 日付パースを安全化、unwrap_or()使用
- locator.rs: unwrap()をunwrap_or_default()に置換
- From/TryFromトレイトは既に統一されており追加抽象化は不要

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

#### #31 ハードコード日付の設定化 ✅
**問題**: 日付がコードに散在
**対策**:
- `AwardPeriod` 構造体に `contains()` メソッド追加
- `user_service.rs`: ハードコード日付を `AwardPeriod::default()` に置換
- `sota.rs`: 同上
- 一度限りのアワードのため環境変数化は不要と判断

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

| カテゴリ | 項目数 | 状態 | 複雑度 | 工数目安 |
|----------|--------|--------|--------|----------|
| セキュリティ | 1 | ✅完了 | - | - |
| エラーハンドリング | 2 | 1/2完了 | 中 | 残8h |
| パフォーマンス | 3 | ✅完了 | - | - |
| ファイル分割 | 2 | ✅完了 | - | - |
| テスト | 1 | ✅完了 | - | - |
| アーキテクチャ | 3 | 1/3完了 | 中-高 | 残20h |
| 設定・保守性 | 2 | 1/2完了 | 低 | 残5h |

## 完了済み項目

1. ~~**#19 SQLインジェクション対策**~~ ✅
2. ~~**#23 unwrap()一掃**~~ ✅
3. ~~**#25 N+1クエリ修正**~~ ✅
4. ~~**#26 Regexキャッシュ**~~ ✅
5. ~~**#7 テストインフラ整備**~~ ✅ (81テスト)
6. ~~**#21 user_service分割**~~ ✅ (1131行→503行、55%削減)
7. ~~**#22 ハンドラ関数重複排除**~~ ✅
8. ~~**#29 CSV変換共通化**~~ ✅
9. ~~**#27 clone()削減**~~ ✅ (117→110箇所)
10. ~~**#31 ハードコード日付の設定化**~~ ✅

## 残り作業（既存）

| 項目 | 優先度 | 複雑度 | 備考 |
|------|--------|--------|------|
| ~~#24 エラーコンテキストの統一~~ | ~~中~~ | ~~中~~ | ✅ 完了 |
| #28 クエリビルダー抽象化 | 低 | 高 | PostGIS使用しないためスキップ |
| #30 Groupingストラテジー抽象化 | 低 | 低 | 効果小、スキップ |
| #32 モジュールドキュメント追加 | 低 | 低 | 必要に応じて |

---

## 新規リファクタリング項目（2024-12-22 レビュー）

### コードベース統計

| 項目 | 値 |
|------|-----|
| 総行数 | 13,889行 |
| テスト数 | 81個 |
| Clippy警告 | 0件 |
| unwrap/expect残存 | 約20箇所 |
| 500行超ファイル | 5個 |

---

### 【セキュリティ - 高優先】

#### #33 エラーレスポンス情報漏洩防止 ✅
**ファイル**: `common/src/error.rs:81-89`
**問題**: `RowNotFound`エラーで内部の`location`情報がレスポンスに含まれていた
**対策**:
- レスポンスには汎用メッセージ「指定されたリソースが見つかりません」のみを返却
- `location`はログにのみ記録（`tracing::error!`）
- 内部詳細の外部露出を防止

#### #34 入力バリデーション強化 ✅
**ファイル**: `api/src/model/param.rs`
**対策完了**:
- `validator`クレート導入
- `GetParam`構造体に`#[validate]`属性追加
  - 数値パラメータ: `range`バリデーション（limit: 1-10000, offset: 0-1000000等）
  - 文字列パラメータ: `length`バリデーション（max 20-100文字）
  - 座標: lat -90〜90, lon -180〜180
- `ValidatedQuery<T>`エクストラクタ作成（自動バリデーション）
- 全APIハンドラで`ValidatedQuery`使用に変更（22箇所）

#### #35 PostGISレガシー関数削除 🟢 LOW
**ファイル**: `adapter/src/database/implement/postgis/querybuilder.rs:220-339`
**問題**: `#[deprecated]`のSQLインジェクション脆弱関数が残存
**状態**: 新APIで対応済み、レガシー関数は未使用
**対策**: 削除（PostGIS使用しないため影響なし）
**複雑度**: 低
**工数**: 1h

---

### 【コード品質 - 中優先】

#### #36 残存unwrap()の置き換え ✅
**対策完了**:

| ファイル | 変更内容 |
|---------|---------|
| `adapter/src/minikvs.rs` | `expect()`に変更（シリアライズ失敗はプログラムエラー） |
| `adapter/src/database/model/pota.rs` | `unwrap_or_default()`使用、`TryFrom`トレイトに変更 |
| `adapter/src/database/model/locator.rs` | `if let`パターンマッチに書き換え |
| `adapter/src/database/implement/sqlite/sota_reference.rs` | `unwrap_or_default()`使用 |
| `adapter/src/database/implement/sqlite/pota_reference.rs` | `try_into()`でエラースキップ |
| `src/bin/app.rs` | `expect()`に変更（設定エラーは起動時検出） |
| `service/src/implement/admin_service.rs` | `filter_map()`+`?`パターンに変更 |

#### #37 clone()最適化 🟢 LOW
**問題**: 115箇所でclone()使用、一部は参照で代替可能
**主な箇所**:
- `postgis/querybuilder.rs:50-56, 119-127`: pattern.clone()を&patternに
- `user_service.rs:144, 170`: spot.clone()
- `admin_service.rs:97, 159`: summit_code.clone()

**対策**: 参照利用に変更
**複雑度**: 低
**工数**: 2h

#### #38 正規表現キャッシング追加 🟢 LOW
**ファイル**: `service/src/implement/user_service.rs:115, 136`
**問題**: 毎回`Regex::new()`を呼び出し
**対策**: `OnceLock`またはLRUキャッシュ導入
**複雑度**: 低
**工数**: 1h

---

### 【テスト - 高優先】

#### #39 APIハンドラテスト実装 ✅ 完了
**問題**: API層のテストカバレッジがゼロ（14個のハンドラ）
**対策**: 純粋関数とビューモデル変換のユニットテストを追加

**Phase 1 完了** (純粋関数・ビューモデル):
- `api/src/handler/auth.rs`: AuthRequestデシリアライズ、Bearerトークン抽出テスト (5テスト)
- `api/src/handler/activation.rs`: apply_common_filters純粋関数テスト (14テスト)
- `api/src/model/param.rs`: GetParamバリデーション、build_findref_queryテスト (27テスト)
- `api/src/model/sota.rs`: SotaRefView/SotaSearchView変換、JSONシリアライズテスト (8テスト)
- `api/src/model/pota.rs`: PotaRefView/PotaSearchView変換、JSONシリアライズテスト (12テスト)

**Phase 2 完了** (追加ビューモデル):
- `api/src/model/activation.rs`: ActivationView変換テスト (6テスト)
- `api/src/model/alerts.rs`: AlertView変換、SotaAlert/PotaAlertデシリアライズテスト (13テスト)
- `api/src/model/spots.rs`: SpotView変換、SotaSpot/PotaSpotデシリアライズテスト (14テスト)
- `api/src/model/search.rs`: SearchResponse/SearchFullResponse/SearchBriefResponse変換テスト (14テスト)
- `api/src/model/aprslog.rs`: AprsLogView/Track変換テスト (14テスト)
- `api/src/model/geomag.rs`: GeomagView変換テスト (7テスト)

**合計124テスト** (API層のみ)
**ワークスペース合計203テスト**

**未対応** (DIモック必要・優先度低):
| ハンドラ | ファイル | 理由 |
|---------|---------|------|
| SOTA ログアップロード | `api/src/handler/sota.rs` | Firebase + Shaku DI依存 |
| POTA ログアップロード | `api/src/handler/pota.rs` | Shaku DI依存 |
| 検索機能 | `api/src/handler/search.rs` | Shaku DI依存 |

**備考**: DIモック構築はShaku統合が複雑なため、E2Eテストでカバーする方針

#### #40 APRSサービステスト 🟡 MEDIUM
**ファイル**: `adapter/src/aprs.rs`, `service/src/implement/aprs_service.rs`
**問題**: 外部連携のテストなし
**対策**: モックサーバーでAPRS接続テスト
**複雑度**: 中
**工数**: 8h

---

### 【アーキテクチャ - 中優先】

#### #41 大規模ファイル分割 🟡 MEDIUM
**500行超ファイル**:

| ファイル | 行数 | 分割案 |
|---------|------|--------|
| `sqlite/sota_reference.rs` | 858 | queries + tests 分離 |
| `sqlite/pota_reference.rs` | 757 | queries + tests 分離 |
| `award_calculator.rs` | 708 | 70%テスト、コアロジック分離 |
| `user_service.rs` | 674 | SOTA/POTA/APRS別サービス化 |
| `admin_service.rs` | 350 | CSV解析ロジック分離 |

**対策**:
```
adapter/src/database/implement/sqlite/
  ├── sota_reference.rs (300行)
  ├── sota_reference_queries.rs (新規)
  └── sota_reference_tests.rs (新規)
```
**複雑度**: 中
**工数**: 12h

#### #42 user_serviceの責務分離 🟡 MEDIUM
**問題**: 1ファイルに5つの責務
1. SOTA ログ管理
2. POTA ログ管理
3. APRS 統合
4. ロケータ検索
5. 地磁気データ取得

**対策**:
```
service/src/implement/
  ├── user_service.rs (汎用, 300行)
  ├── sota_log_service.rs (新規, 150行)
  ├── pota_log_service.rs (新規, 150行)
  └── reference_search_service.rs (新規, 100行)
```
**複雑度**: 高
**工数**: 16h

---

### 【ドキュメント - 低優先】

#### #43 OpenAPI/Swagger導入 🟢 LOW
**問題**: APIドキュメント不足（カバレッジ71%）
**対策**: `utoipa`クレートでOpenAPI仕様生成
**複雑度**: 中
**工数**: 8h

#### #44 データベーススキーマドキュメント 🟢 LOW
**問題**: スキーマ説明ドキュメントなし
**対策**: `migrations/`からER図・仕様書生成
**複雑度**: 低
**工数**: 4h

---

## 優先順位サマリー（更新版）

| 優先度 | 項目 | 工数 | 次期スプリント候補 |
|--------|------|------|------------------|
| ✅ | #33 エラー情報漏洩防止 | 完了 | |
| ✅ | #34 入力バリデーション | 完了 | |
| ✅ | #36 unwrap()置き換え | 完了 | |
| ✅ | #24 エラーコンテキスト統一 | 完了 | |
| ✅ | #39 APIハンドラテスト（Phase 1） | 完了 | 66テスト追加 |
| 🔴 高 | #39 APIハンドラテスト（Phase 2） | 8h | DIモック構築 |
| 🟡 中 | #40 APRSテスト | 8h | |
| 🟡 中 | #41 ファイル分割 | 12h | |
| 🟡 中 | #42 user_service責務分離 | 16h | |
| 🟢 低 | #35 PostGISレガシー削除 | 1h | ✓ |
| 🟢 低 | #37 clone()最適化 | 2h | |
| 🟢 低 | #38 Regexキャッシング | 1h | |
| 🟢 低 | #43 OpenAPI導入 | 8h | |
| 🟢 低 | #44 スキーマドキュメント | 4h | |

---

## 次期スプリント推奨

1. ~~**#33 エラー情報漏洩防止** (1h)~~ ✅ 完了
2. ~~**#36 unwrap()置き換え** (3h)~~ ✅ 完了
3. ~~**#34 入力バリデーション** (4h)~~ ✅ 完了
4. ~~**#24 エラーコンテキスト統一** (2h)~~ ✅ 完了
5. ~~**#39 APIハンドラテスト Phase 1** (~8h)~~ ✅ 完了
   - auth.rs: デシリアライズ、トークン抽出テスト
   - param.rs: バリデーション、クエリビルダーテスト (27テスト)
   - activation.rs: 純粋関数テスト (14テスト)
   - sota.rs/pota.rs: ビューモデル変換テスト (20テスト)
6. **#35 PostGISレガシー削除** (1h) - セキュリティ
7. **#39 APIハンドラテスト Phase 2** (8h) - モックDI

合計: ~18h完了済み
