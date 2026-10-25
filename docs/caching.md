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

## Cloudflare のキャッシュルール(必須。設定済み: 2026-10-25)

Cloudflare は、JSON の API 応答を**既定ではキャッシュしない**(`cf-cache-status: DYNAMIC`)。ヘッダーを付けるだけでは、Fly.io までの要求は減らない。
次のキャッシュルールを足す(ダッシュボード → `sotalive.net` → キャッシュ → Cache Rules):

- 条件(式): `(http.host eq "sotaapp2.sotalive.net" and starts_with(http.request.uri.path, "/api/v2/") and http.request.method eq "GET")`
- キャッシュの適格性(Cache eligibility): **適格(Eligible for cache)**
- エッジ TTL(Edge TTL): **キャッシュ制御ヘッダーがあれば従い、なければキャッシュをバイパス**(Use cache-control header if present, bypass cache if not)
- ブラウザー TTL(Browser TTL): **Respect origin TTL**(オリジンの TTL を尊重)
- ほかの項目(クエリ文字列、キャッシュキー)は変えない(既定でクエリを含む URL 全体がキャッシュの単位)

注意すること:

- **エッジ TTL は「ヘッダーがなければバイパス」にする。** 「なければ Cloudflare の既定の時間でキャッシュ」にすると、ヘッダーを付けていない `/sota/maps`(地図の本体)や `/pota` まで数時間キャッシュされ、古い地図を配る恐れがある。
- **ブラウザー TTL を「Respect origin TTL」にする。** 別の選択肢(固定の時間)を選ぶと、Cloudflare が応答の `Cache-Control` を書き換える。実際に、最初の設定で `max-age=14400`(4 時間)に書き換わり、MyACT のブラウザーがスポットを最大 4 時間使い回す状態になった。直したあとは、サーバーの `max-age`(スポット 30 秒)がそのまま返る。

確認: 同じ URL を 2 回呼び、2 回目の応答に `cf-cache-status: HIT`(1 回目は `MISS`)が出れば有効。`cache-control` がサーバーの宣言(`max-age=30` など)のままであることも見る。

```bash
curl -s -D - -o /dev/null "https://sotaapp2.sotalive.net/api/v2/activation/spots?pat_ref=%5E(JA%7CJP-)" | grep -iE "cache-control|cf-cache-status|^age"
```

### 本番での確認結果(2026-10-25)

| 種類 | `cf-cache-status` |
|---|---|
| スポット・アラート・軌跡・地磁気・検索・JCC/JCG・地図の版 | 2 回目が `HIT`。`cache-control` はサーバーの宣言のまま |
| `log_id=` つきのスポット | `BYPASS`(`private`) |
| `/sota/maps`・`/map`・`/pota`・`/activation/aprs/log`・`/pota/log-stat`・管理系・エラー(400・401) | `BYPASS` |

スポットは、約 30 秒で `EXPIRED` になり、取り直される。

## 反映の遅れ

直したものが利用者に届くまでの最大の遅れは、エンドポイントの `max-age` ぶん(スポットは 30 秒、地図の版は 60 秒、JCC/JCG の表は 1 時間)。
JCC/JCG の表を取り込んですぐに確かめたいときは、Cloudflare のキャッシュを消す(「キャッシュ → 設定 → キャッシュの消去」で URL 指定)。
