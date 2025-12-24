# メモリ効率の良いリスト更新処理 テスト計画

## 概要

`update_summit_list_from_file` および `update_pota_park_list_from_file` の実装に対するリアルデータテスト計画。

### 対象コミット
- `cd387af` - メモリ効率の良いリスト更新処理を実装

### テスト環境
- 本番環境: Fly.io 256MB インスタンス
- ローカル環境: 開発マシン（メモリ制限シミュレーション可能）

---

## 1. メモリ使用量評価

### 1.1 ベースライン測定（旧実装）

| 項目 | 測定方法 | 期待値 |
|------|----------|--------|
| SOTA更新時ピークメモリ | `/proc/self/status` の VmRSS | ~110MB |
| POTA更新時ピークメモリ | 同上 | ~70MB |

### 1.2 新実装測定

| 項目 | 測定方法 | 期待値 |
|------|----------|--------|
| SOTA更新時ピークメモリ | `/proc/self/status` の VmRSS | < 20MB |
| POTA更新時ピークメモリ | 同上 | < 15MB |

### 1.3 測定手順

```bash
# ローカルでメモリ制限付き実行
# cgroups v2 を使用してメモリ制限をシミュレート
systemd-run --user --scope -p MemoryMax=256M ./target/release/app

# または Docker で制限
docker run --memory=256m -it sotaapp2:test
```

### 1.4 メモリプロファイリング

```rust
// 各フェーズでのメモリ使用量をログ出力（デバッグ用）
fn log_memory_usage(phase: &str) {
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                tracing::info!("{}: {}", phase, line);
            }
        }
    }
}
```

---

## 2. 速度・パフォーマンス評価

### 2.1 測定項目

| フェーズ | SOTA期待時間 | POTA期待時間 |
|----------|--------------|--------------|
| ダウンロード | ~5-10秒 | ~3-5秒 |
| Pass 1 (ハッシュ構築) | ~2-3秒 | ~1-2秒 |
| DB既存データ取得 | ~1-2秒 | ~1秒 |
| Pass 2 (差分計算) | < 1秒 | < 1秒 |
| Pass 3 (更新対象読込) | ~1-2秒 | ~1秒 |
| DB upsert | ~5-10秒 | ~3-5秒 |
| DB delete | < 1秒 | N/A |
| **合計** | ~15-30秒 | ~10-15秒 |

### 2.2 測定手順

```rust
// 各フェーズの開始・終了時刻をログ出力
let start = std::time::Instant::now();
// ... 処理 ...
tracing::info!("Phase X completed in {:?}", start.elapsed());
```

### 2.3 比較基準

- 旧実装との比較（許容範囲: 旧実装の 1.5倍以内）
- ネットワーク遅延の影響を考慮（複数回測定の平均を使用）

---

## 3. バグ検証

### 3.1 SOTA: 無効サミット削除の検証

#### テストケース 3.1.1: 新規無効サミットの削除

**前提条件:**
- DBに有効なサミット `JA/TK-001` が存在
- CSVで `JA/TK-001` が `ValidTo` 過去日付で無効化

**期待結果:**
- `JA/TK-001` がDBから削除される
- ログに削除件数が出力される

**検証SQL:**
```sql
-- 実行前
SELECT COUNT(*) FROM sota_references WHERE summit_code = 'JA/TK-001';
-- 実行後
SELECT COUNT(*) FROM sota_references WHERE summit_code = 'JA/TK-001';
-- 結果: 0
```

#### テストケース 3.1.2: 有効サミットは保持

**前提条件:**
- DBに有効なサミット `JA/TK-002` が存在
- CSVで `JA/TK-002` が引き続き有効

**期待結果:**
- `JA/TK-002` がDBに保持される
- 内容が更新される（変更があれば）

#### テストケース 3.1.3: 新規サミットの追加

**前提条件:**
- DBにサミット `JA/TK-999` が存在しない
- CSVに `JA/TK-999` が有効として存在

**期待結果:**
- `JA/TK-999` がDBに追加される

### 3.2 POTA: inactiveパーク保持の検証

#### テストケース 3.2.1: inactiveパークは削除されない

**前提条件:**
- DBにアクティブなパーク `JA-0001` が存在
- CSVで `JA-0001` が `active: 0` (inactive)

**期待結果:**
- `JA-0001` がDBに保持される（削除されない）
- `active` フラグが `0` に更新される

**検証SQL:**
```sql
SELECT park_code, active FROM pota_references WHERE park_code = 'JA-0001';
-- 結果: JA-0001, 0
```

#### テストケース 3.2.2: activeに戻ったパークの更新

**前提条件:**
- DBに inactive なパーク `JA-0002` が存在
- CSVで `JA-0002` が `active: 1` に戻る

**期待結果:**
- `JA-0002` の `active` フラグが `1` に更新される

### 3.3 データ整合性検証

#### テストケース 3.3.1: ハッシュ比較の正確性

**手順:**
1. 初回実行: 全件 upsert
2. 即時再実行: 変更なしで upsert 0件

**検証:**
```sql
-- 初回実行後
SELECT COUNT(*) FROM sota_references;  -- ~170,000

-- 再実行のログ
-- "0 summits updated" が出力されること
```

#### テストケース 3.3.2: activation_count の更新

**前提条件:**
- DBのサミット `JA/TK-003` の `activation_count = 10`
- CSVで `activation_count = 15` に更新

**期待結果:**
- DBの `activation_count` が `15` に更新される

---

## 4. 実行チェックリスト

### 4.1 ローカルテスト

- [ ] ユニットテスト全件パス (`cargo make test`)
- [ ] ローカル環境でSOTA更新実行
- [ ] ローカル環境でPOTA更新実行
- [ ] メモリ使用量ログ確認

### 4.2 ステージング/本番テスト

- [ ] 256MB制限環境でSOTA更新実行
- [ ] 256MB制限環境でPOTA更新実行
- [ ] OOM発生なしを確認
- [ ] 更新件数がログに出力されることを確認

### 4.3 バグ検証

- [ ] SOTA無効サミット削除確認
- [ ] POTAinactiveパーク保持確認
- [ ] データ整合性確認（再実行で0件更新）

### 4.4 パフォーマンス確認

- [ ] SOTA更新時間が許容範囲内（< 30秒）
- [ ] POTA更新時間が許容範囲内（< 15秒）

---

## 5. ロールバック手順

問題発生時のロールバック:

```bash
# 前のコミットに戻す
git revert cd387af

# または旧実装を一時的に使用
# update_summit_list / import_pota_park_list を呼び出すように変更
```

---

## 6. 監視項目（本番運用後）

- Fly.io メトリクス: メモリ使用量推移
- アプリケーションログ: 更新件数、エラー有無
- cron ジョブ成功率

---

## 変更履歴

| 日付 | 内容 |
|------|------|
| 2024-12-25 | 初版作成 |
