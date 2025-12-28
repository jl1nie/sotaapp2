# Cloudflare移行計画: sotalive-vercel → Cloudflare Pages + Full Rust

## 概要

フロントエンド(sotalive-vercel)をVercelからCloudflare Pages + Workers へ移行し、Python Functions (~3100行) を全てRust化してsotaapp2に統合する。

## 現状アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                      Vercel                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  静的ファイル: myact/, logconv/, myqth/                 │ │
│  │  Python Functions: logconv.py, fleonline.py,           │ │
│  │                    convutil.py, wspr.py (~3100行)      │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              ↓ API呼び出し
┌─────────────────────────────────────────────────────────────┐
│                      Fly.io                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  sotaapp2 (Rust/Axum) - REST API + SQLite              │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 目標アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                  Cloudflare Pages                            │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  静的ファイル: myact/, logconv/, myqth/                 │ │
│  │  (全APIリクエストはFly.ioへプロキシ)                    │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                              ↓ 全API
┌─────────────────────────────────────────────────────────────┐
│                      Fly.io                                  │
│  ┌────────────────────────────────────────────────────────┐ │
│  │  sotaapp2 (Rust/Axum)                                   │ │
│  │  既存API + NEW:                                         │ │
│  │  - /api/v2/logconv/* (ADIF/CSV変換)                    │ │
│  │  - /api/v2/fleonline/* (FLEパース)                     │ │
│  │  - /api/v2/wspr/svg (WSPR SVG生成)                     │ │
│  └────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 移行オプション比較

### Option A: Python Workers移行
- 移行難易度: 低（コードほぼそのまま）
- 実行性能: 中（Pyodideオーバーヘッド）
- 保守性: 中（Python/Rustの2言語）
- 制約: matplotlib不可、10ms CPU制限

### Option B: Full Rust移行（採用）
- 移行難易度: 中（ロジック移植が必要）
- 実行性能: 高（ネイティブ速度）
- 保守性: 高（単一言語・単一リポジトリ）
- 制約: なし

## Full Rust移行の詳細

### 移行対象コード

| ファイル | 行数 | 機能 | Rust移行難易度 |
|---------|------|------|---------------|
| convutil.py | 1297 | ADIF/CSVパース、変換 | 中（既存ADIFクレートあり） |
| fleonline.py | 1416 | FLE形式パーサー | 中（パーサー実装必要） |
| logconv.py | 229 | エントリポイント | 低（ハンドラー追加） |
| wspr.py | 172 | SVGグラフ生成 | 低（plottersで実装） |
| **合計** | **3114** | | |

### 必要なRustクレート
- `csv` - CSVパース（既存）
- `zip` - ZIP生成
- `plotters` - SVG生成（wspr用）
- `chrono` - 日時処理（既存）
- `regex` - 正規表現（既存）
- ADIFパース（要調査 or 自前実装）

## 移行計画

### Phase 1: Cloudflare Pages移行（静的ファイル）

1. Cloudflareプロジェクト作成
2. wrangler.toml設定
3. 静的ファイル（myact/, logconv/, myqth/）をPages配信
4. Workers Functionsでsotaapp2へのプロキシ実装
5. カスタムドメイン設定（*.sotalive.net）
6. DNS切り替え

**成果物**:
- `/functions/api/[[path]].ts` - Fly.ioへのプロキシ
- `wrangler.toml` - Cloudflare設定

### Phase 2: wspr.py Rust移行

wspr.py（WSPR SVG生成）をsotaapp2バックエンドに統合。

**実装**:
```
sotaapp2/
├── api/src/handler/wspr.rs      # 新規
├── service/src/implement/wspr_service.rs  # 新規
└── Cargo.toml                   # plotters追加
```

**エンドポイント**: `POST /api/v2/wspr/svg`

### Phase 3: logconv Full Rust移行

Python Functions → Rust (sotaapp2に統合)

| ファイル | 行数 | Rust実装 |
|---------|------|---------|
| convutil.py | 1297 | service/src/implement/logconv/ |
| fleonline.py | 1416 | service/src/implement/fle/ |
| logconv.py | 229 | api/src/handler/logconv.rs |

**段階的実装**:

1. **Step 3a: convutil移行**
   - ADIF/CSVパース実装
   - HAMLOG形式変換
   - SOTA/POTA/WWFF形式出力

2. **Step 3b: FLEパーサー移行**
   - トークナイザー実装
   - ステートマシン実装
   - 各種出力形式対応

3. **Step 3c: ハンドラー統合**
   - マルチパート対応
   - ZIP出力
   - エラーハンドリング

**新規エンドポイント**:
- `POST /api/v2/logconv/hamlog` - HAMLOG CSV変換
- `POST /api/v2/logconv/fleonline` - FLE形式変換
- `POST /api/v2/logconv/wspr` → Phase 2で実装済み

### Phase 4: Vercel停止・最適化

1. Vercelプロジェクト停止
2. Cloudflareキャッシュルール最適化
3. モニタリング設定
4. ドキュメント整備

### Phase 5: モノレポ化検討（後日）

Turborepoでsotaapp2 + sotalive + adminを統合するか検討。

## 工数見積（Full Rust移行）

| Phase | 期間 | 主要タスク | ステータス |
|-------|------|-----------|-----------|
| Phase 1 | 1-2週間 | Pages移行、プロキシ | **完了** ✅ |
| Phase 2 | 1週間 | wspr.py Rust移行 (plotters) | **完了** ✅ |
| Phase 3a | 2-3週間 | convutil Rust移行 (ADIF/CSV) | **完了** ✅ |
| Phase 3b | 2-3週間 | FLEパーサー Rust移行 | **完了** ✅ |
| Phase 3c | 1週間 | ハンドラー統合・テスト | **完了** ✅ |
| Phase 4 | 1週間 | Vercel停止・最適化 | 未着手 |
| Phase 5 | 後日 | モノレポ検討（オプション） | 未着手 |
| **合計** | **8-11週間** | | |

### 完了した作業（2024-12-28）

#### Phase 3b: FLEパーサー Rust移行
- `service/src/implement/fle/mod.rs` - FLEモジュール定義、FSM設計ドキュメント
- `service/src/implement/fle/types.rs` - 共通型（FleEnvironment, RstValue, FleQsoRecord等）
- `service/src/implement/fle/tokenizer.rs` - トークナイザー（Token enum, tokenize関数）
- `service/src/implement/fle/compiler.rs` - FSMベースコンパイラ（State: Norm/Freq/RstSent/RstRcvd）
- `service/src/implement/fle/output.rs` - 出力フォーマッタ（SOTA CSV/POTA ADIF/HAMLOG CSV/AirHam/ZLOG）
- `api/src/handler/fle.rs` - APIエンドポイント (`/api/v2/fle/compile`, `/api/v2/fle/generate`)
- 19個のユニットテスト追加
- Clippy警告最小化

#### Phase 3a: convutil.py Rust移行
- `service/src/implement/logconv/mod.rs` - モジュール定義
- `service/src/implement/logconv/types.rs` - 共通型、周波数/バンド変換テーブル、モード変換
- `service/src/implement/logconv/adif.rs` - ADIFパーサー
- `service/src/implement/logconv/hamlog.rs` - HAMLOG CSV / HamLog iOSパーサー
- `service/src/implement/logconv/converter.rs` - SOTA/POTA/WWFF出力変換、ZIP生成
- `api/src/handler/logconv.rs` - APIエンドポイント (`/api/v2/logconv/*`)
- 23個のユニットテスト追加
- Clippy警告0件

#### Phase 1: Cloudflare Pages移行
- `/home/minoru/src/sotalive-cloudflare/` プロジェクト作成
- `wrangler.toml` - Cloudflare Pages設定
- `functions/api/[[path]].ts` - Fly.ioへのAPIプロキシWorker
- `functions/_middleware.ts` - サブドメインルーティング（myact/logconv/myqth）
- 静的アセットをsotalive-vercelからコピー
- TypeScript/package.json設定
- コミット: `efe7e8e`

#### Phase 2: wspr.py Rust移行
- `service/src/implement/wspr_service.rs` - WSPRスポットデータからSVG生成
- `api/src/handler/wspr.rs` - `POST /api/v2/wspr/svg` エンドポイント
- `plotters`クレートを使用してmatplotlib相当のグラフ描画
- 複数プロット、散布図+平均線、カラー対応
- コミット: `b5fb8f6`

-------|------|-----------|
| Phase 1 | 1-2週間 | Pages移行、プロキシ |
| Phase 2 | 1週間 | wspr.py Rust移行 (plotters) |
| Phase 3a | 2-3週間 | convutil Rust移行 (ADIF/CSV) |
| Phase 3b | 2-3週間 | FLEパーサー Rust移行 |
| Phase 3c | 1週間 | ハンドラー統合・テスト |
| Phase 4 | 1週間 | Vercel停止・最適化 |
| Phase 5 | 後日 | モノレポ検討（オプション） |
| **合計** | **8-11週間** | |

## リスクと対策（Full Rust移行）

| リスク | 対策 |
|--------|------|
| Rust移行の工数超過 | 段階的実装、各Phaseで動作確認 |
| FLEパーサーの複雑性 | 既存Pythonと同じテストケースで検証 |
| DNS切り替え障害 | 段階的切り替え、ロールバック手順準備 |
| ADIFパーサー実装 | 既存クレート調査、なければシンプル実装 |
| 並行運用期間 | Phase 3完了までVercel維持（フォールバック） |

## 変更対象ファイル

### 新規作成（sotalive-cloudflare）
- `wrangler.toml` - Cloudflare設定
- `functions/api/[[path]].ts` - Fly.ioへのプロキシWorker

### 新規作成（sotaapp2 - Phase 2: wspr）
- `api/src/handler/wspr.rs` - WSPRハンドラー
- `service/src/implement/wspr_service.rs` - WSPR SVG生成ロジック

### 新規作成（sotaapp2 - Phase 3: logconv）
- `api/src/handler/logconv.rs` - logconvエントリポイント
- `service/src/implement/logconv/mod.rs` - convutil移植
- `service/src/implement/logconv/adif.rs` - ADIFパーサー
- `service/src/implement/logconv/hamlog.rs` - HAMLOG変換
- `service/src/implement/logconv/sota.rs` - SOTA形式出力
- `service/src/implement/logconv/pota.rs` - POTA形式出力
- `service/src/implement/fle/mod.rs` - FLEパーサー
- `service/src/implement/fle/tokenizer.rs` - トークナイザー
- `service/src/implement/fle/compiler.rs` - FLEコンパイラ

### 変更（sotaapp2）
- `api/src/handler/mod.rs` - wspr/logconvハンドラー追加
- `service/src/services.rs` - LogconvService trait追加
- `Cargo.toml` - zip, plotters依存追加

## 採用理由

1. **シンプル** - Cloudflare/Vercelへの依存なし
2. **長期保守** - 単一言語・単一デプロイ
3. **制約回避** - CPU制限・パッケージ制限の心配不要
4. **既存インフラ活用** - Fly.io既存環境を活用
5. **型安全** - コンパイル時エラー検出

## Phase 5: モノレポ＋新サブドメイン戦略（評価済み）

### 提案構成

```
現行（3ヶ月間維持）
sotalive.net (Vercel)
├── myact.sotalive.net
├── logconv.sotalive.net
└── wspr.sotalive.net

新規（Cloudflare Pages）
├── myact2.sotalive.net
├── logconv2.sotalive.net
└── wspr2.sotalive.net

バックエンド（共通）
└── api.sotalive.net または sotaapp2.fly.dev
```

### モノレポ構成案

```
sotalive-monorepo/
├── apps/
│   ├── myact/        # MyActivation (SvelteKit)
│   ├── logconv/      # LogConverter (SvelteKit)
│   └── wspr/         # WSPR (SvelteKit)
├── packages/
│   └── shared/       # 共通コンポーネント・ユーティリティ
├── wrangler.toml     # Cloudflare Pages設定
└── turbo.json        # Turborepo設定
```

### メリット

1. **ゼロダウンタイム移行**: 既存サービス維持しながら新サービス並行運用
2. **段階的ユーザー移行**: 新URLを案内し自主的に移行
3. **ロールバック容易**: 問題時は旧サービス稼働中
4. **バックエンド共通化**: sotaapp2で実装済みAPI共通利用
5. **管理簡素化**: 全フロントエンドを一元管理

### 移行手順

1. **モノレポ作成**: Turborepo + SvelteKit構成
2. **共通パッケージ整備**: UI/ユーティリティ共通化
3. **Cloudflare Pages設定**: 新サブドメイン（*2.sotalive.net）
4. **DNS設定**: Cloudflareで新サブドメイン追加
5. **並行運用開始**: ユーザーに新URL案内
6. **旧サービス停止**: SSL証明書期限（約3ヶ月後）に合わせて停止

### 評価結果

**この戦略は妥当**。理由：
- バックエンドAPI（sotaapp2）は既に3機能すべてRust実装済み
- フロントエンドモノレポ化で共通コンポーネント再利用可能
- Cloudflare Pagesは複数サブドメインを1プロジェクトで対応可能
- 既存ユーザーへの影響最小限で移行可能

## 作成日
2024-12-28（最終更新: 2024-12-28 Phase 5戦略追記）
