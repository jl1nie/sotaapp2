# 公開 API のキャッシュ(`Cache-Control`)

本番は Fly.io の小さなマシン(`shared-cpu-1x`、メモリ 256 MB)1 台。MyACT は、開いている間、スポットを毎分呼ぶ。
データは 90〜120 秒に 1 回しか変わらないので、開いている人数 × 毎分、同じ応答を組み立て直すのは無駄になる。

更新の周期を知っているのは API なので、**API が `Cache-Control` で「この応答は何秒新しい」と宣言**し、
Cloudflare がそれに従って応答を覚える(`api/src/handler/cache.rs` の `cached`)。

## 付けるもの

| エンドポイント | `Cache-Control` | 理由 |
|---|---|---|
| `/activation/spots*`・`/activation/alerts*`・`/activation/aprs/track`、`/sota/spots`・`/sota/alerts`、`/pota/spots`・`/pota/alerts` | `public, max-age=30` | 取得は `SPOT_INTERVAL`(90〜120 秒)・`ALERT_INTERVAL` ごと |
| `/propagation/geomag` | `public, max-age=600` | 3 時間ごとの更新 |
| `/search`・`/search/full`・`/search/brief`、`/sota/summits/search`、`/pota/parks/search` | `public, max-age=300` | 山・公園の一覧はほとんど変わらない |
| `/locator/jcc-jcg` | `public, max-age=3600` | 管理者が CSV を取り込んだときだけ変わる |
| `/sota/maps/versions` | `public, max-age=60` | 管理画面で答えを直したときだけ変わる |

- 付けるのは、**GET で成功(2xx)した応答だけ**。エラーは覚えさせない。すでに `Cache-Control` がある応答、`Set-Cookie` がある応答には付けない。
- **利用者ごとの応答**(POTA ハンターのログ `log_id=` つきの要求)は `private`(共有のキャッシュに載せない)。
- キャッシュの単位は URL 全体(クエリを含む)。`pat_ref` などが違えば、別のキャッシュ。

## 付けないもの

- **`/sota/maps`(地域の地図の本体)・`/sota/summits/{code}/map`・`/sota/summits/{code}/pota`**: FLEonline は、版(`/sota/maps/versions`)が変わったときだけ地図を取り直す。地図の本体を覚えさせると、版の問い合わせだけが新しく、古い地図を新しい版として端末に保存してしまう恐れがある。
- 管理系(`/admin`・見直し・取り込み)、ログ、認証、`/activation/aprs/log`。

## Cloudflare のキャッシュルール(必須)

Cloudflare は、JSON の API 応答を**既定ではキャッシュしない**(`cf-cache-status: DYNAMIC`)。ヘッダーを付けるだけでは、Fly.io までの要求は減らない。
次のキャッシュルールを足す(ダッシュボード → `sotalive.net` → キャッシュ → キャッシュルール):

- 条件: ホスト名が `sotaapp2.sotalive.net` で、URI パスが `/api/v2/` で始まり、メソッドが GET
- キャッシュの適格性: **キャッシュ対象**
- エッジ TTL: **キャッシュ制御ヘッダーがあれば従い、なければキャッシュしない**(「オリジンのキャッシュ制御を使用し、なければバイパス」)
- ブラウザー TTL: **オリジンに従う**

`Cache-Control` を付けていないエンドポイントは、この設定でも覚えられない(バイパス)。

確認: 同じ URL を 2 回呼び、2 回目の応答に `cf-cache-status: HIT`(1 回目は `MISS`)が出れば有効。

```bash
curl -s -D - -o /dev/null "https://sotaapp2.sotalive.net/api/v2/activation/spots?pat_ref=%5E(JA%7CJP-)" | grep -iE "cache-control|cf-cache-status|^age"
```

## 反映の遅れ

直したものが利用者に届くまでの最大の遅れは、エンドポイントの `max-age` ぶん(スポットは 30 秒、地図の版は 60 秒、JCC/JCG の表は 1 時間)。
JCC/JCG の表を取り込んですぐに確かめたいときは、Cloudflare のキャッシュを消す(「キャッシュ → 設定 → キャッシュの消去」で URL 指定)。
