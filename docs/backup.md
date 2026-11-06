# DB のバックアップと復元

本番の DB(SQLite、Fly.io のボリューム `/data/sotaapp2.db`)の写しを、2 か所に取る。

| 種類 | いつ | どこへ | 保存期間 |
|---|---|---|---|
| デプロイ前 | `deploy.yml` を実行したとき | GitHub Actions の artifact(`db-backup-<run>`) | 30 日 |
| 定期 | 毎週 月曜 03:00 JST(`backup.yml`。手動でも実行できる) | Cloudflare R2 のバケット `sotaapp2-backup` の `weekly/` | 90 日(バケットのライフサイクルで自動削除) |

定期の保存は、`weekly/sotaapp2_YYYYMMDD_HHMMSS.db.gz`。取るたびに、gzip を解いて SQLite の整合性(`PRAGMA integrity_check`)を確かめ、R2 に置いたあとサイズが同じことを確かめる。

## 必要な設定(一度だけ)

GitHub の Secrets(リポジトリの Settings → Secrets and variables → Actions):

| 名前 | 内容 |
|---|---|
| `FLY_API_TOKEN` | `deploy.yml` と同じ |
| `R2_ENDPOINT_URL` | `https://<Cloudflare のアカウント ID>.r2.cloudflarestorage.com`(末尾にバケット名や `/` を付けない) |
| `R2_ACCESS_KEY_ID`・`R2_SECRET_ACCESS_KEY` | R2 の API トークン。権限は Object Read & Write、対象はバケット `sotaapp2-backup` だけ(ダッシュボード → R2 → Manage R2 API Tokens) |

バケットとライフサイクル(`weekly/` は 90 日で削除)は、次のコマンドで作った:

```bash
npx wrangler r2 bucket create sotaapp2-backup
npx wrangler r2 bucket lifecycle add sotaapp2-backup expire-weekly-90d weekly/ --expire-days 90
```

## 復元の手順

1. 戻したい写しを選ぶ(R2 のダッシュボード、または `aws s3 ls s3://sotaapp2-backup/weekly/ --endpoint-url "$R2_ENDPOINT_URL"`)。
2. 手元に取り、解く:
   ```bash
   aws s3 cp s3://sotaapp2-backup/weekly/sotaapp2_YYYYMMDD_HHMMSS.db.gz . --endpoint-url "$R2_ENDPOINT_URL"
   gunzip sotaapp2_YYYYMMDD_HHMMSS.db.gz
   sqlite3 sotaapp2_YYYYMMDD_HHMMSS.db "PRAGMA integrity_check"   # ok と出ること
   ```
3. 本番に戻すときは、アプリを止めてから、ボリュームに置き換える(**本番のデータが上書きされる**ので、先に今の DB の写しを別に取っておく):
   ```bash
   fly ssh console -C "cp /data/sotaapp2.db /data/before_restore.db"
   fly ssh sftp shell      # put <解いた .db> /data/sotaapp2.db  (sftp が止まることがある。止まったら再実行)
   fly machine restart <マシン ID>
   ```
4. `/api/v2/health` と、スポット・検索が返ることを確かめる。

## 復元の練習

バックアップは、戻せて初めて役に立つ。年に 1〜2 回は、手順の 2(手元に取って、`integrity_check` が `ok` になること、主なテーブルの件数が妥当なこと)まで試す。本番への置き換え(3)は、実際に必要なときだけ。
