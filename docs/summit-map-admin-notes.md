# 山頂の地図と POTA の判定の管理画面: 検討の経緯

記録: 2026-10-25。続きの作業はここから始める(「今の状態と次にやること」へ)。

- 元の計画: [summit-map-admin-plan.md](summit-map-admin-plan.md)
- 詳細設計(実装済み): [summit-map-admin-design.md](summit-map-admin-design.md)
- バッチ・移行の手順書(sotalive-monorepo): `docs/summit-batch.md`

## 進め方の決まり(利用者の指示)

- **エージェントの進め方は [agent-workflow.md](agent-workflow.md)。** 2026-10-25 に、最初の「実装はいつも Sonnet のサブエージェント」から、この標準に変えた(PR #41)。設計は plan mode の Opus、承認したら同じループの Sonnet で実装する。
- **設計が終わったら、一度利用者に確認をとる。** 確認がとれるまで実装に入らない。
- 返事は日本語で。相談のときは実装しない(「直して」「PR を作って」と言われたら実装する)。
- コミット・PR は、利用者に知らせてから。

## 目的(検討の途中ではっきりしたこと)

最初の依頼は「FLEonline の管理画面の実装」だった。これを計画(summit-map-admin-plan.md)のことと確かめ、範囲は計画 8 章の 1〜4 で始めた。

その後、利用者が本当にやりたいことが分かった。

> FLEonline の利用者から地図の記載に問題が届いたときの、直し方を簡単にしたい。今の Claude Code の Artifact(判定表 `~/sotaapp2-backups/sota_pota_20261024/rereview/index.html`)は、利用者しか編集できない。

- 直す人は、今はいない。のちに数人。
- 報告は DM で届く。アプリの中の報告の受け口は、利用者が増えてからでよい。
- 管理者は利用者を含めて 2 人。管理の API を呼んでいるのは管理画面だけ(スクリプトや cron はない)。

## 検討したこと(問いと答えの順)

### 1. 地図の出力と、領域地図(MyACT の公園区域)との差分を、DB でどう持つか

最初の設計にあったのは、下地と配信用の GeoJSON と、`data_version` という 1 本の文字列だけだった。足りないことが 3 つあった。

- 配信用の地図を、いつ・どの状態から作ったかが分からない。
- 確定したときの版が残らない。
- 領域地図とのずれが見えない。山頂の地図は簡略化しない `jaffpota-annotated-v24-full`、MyACT は 2 m 簡略化した v24 から作っている。React 版の MyACT はまだ v22。

案は 3 つ出した。

| 案 | 中身 | 決めたこと |
|---|---|---|
| A | `summit_maps` に `base_rev`(下地を入れたときの推定の `input_rev`)・`served_at` を足す。地域ごとの版の API にも使う | 入れる |
| B | 版を `park_version`・`muni_version`・`munitable_version` に分け、確定したときの版を `summit_reviews` に写す | 入れる |
| C | 公園ごとの区域の指紋の表 `park_area_versions(version, pota_code, geom_sha, area_m2)`。jaffpota の release で出して取り込む | 後回し(バッチの変更と一緒に) |

full と 2 m 簡略化の差(最大 2 m)は、境界付近の判定の幅(FLE の `EDGE_M` = 20 m)に収まる。なので、形の差は DB に持たない。

### 2. 地図を書くのはオフラインか

そのとおり。分担は次のとおり。

| 段階 | どこで | 中身 |
|---|---|---|
| 形を作る | 手元のバッチ(オフライン) | ゾーン・市区町村境・公園の区域・文字の位置 → 山頂ごとの GeoJSON(下地) |
| 答えを差し込む | サーバー | JSON の中身の書き換えだけ(muni の `pota`、「一部」でない park を外す、判定用の層を外す)。座標には触らない |
| 描く | ブラウザー | FLEonline(端末に保存し、電波のない山頂で GPS と重ねる)と管理画面が SVG にする |

本番(Fly.io)は CPU の割り当てが小さく、区域データや標高データも置いていない。そのため、形の誤り(区域・境界・ゾーン・山頂の位置)は、管理画面では直せない。答えで直したうえでメモを残し、元のデータを直して手元でバッチを流し直す。

### 3. Artifact の判定表のような、注釈付きの判断画面は出せるか

出せる。判定表も、注釈と SVG の座標は手元の Python(`explore/review2_data.py`)が計算し、ページはそれを描くだけだった。管理画面でも、バッチが注釈を DB に入れ、画面が描けば同じものになる。

`rereview/data3.json` と突き合わせて足りなかった注釈は、詳細設計で列を足した。

- 推定の組: `tot_px`、`dist_m`、`shallow`、`hint`、`needs_review`
- 山頂: `reasons`、`change_note`
- 詳細の返り値: `activationCount`、`inCityJ`

ただし、今のバッチの出力(6 列の CSV と、答えを焼き込んだ地図)には、注釈も判定用の層(`judge_park`・`peak`)もない。バッチを変える(計画 6 章)までは、画面に出るのはゾーン・市区町村・「一部」の斜線と推定の答え、外部の地図へのリンクだけになる。報告を受けて 1 山頂を直すには、これで足りると判断した。

### 4. 直す人を増やすときの認証

調べて分かったこと:

- Firebase 認証を使っているのは管理画面だけ(FLEonline・MyACT にはログインがない)。
- `with_auth` は役割を区別していない。Firebase のアカウントを持つ人は、全員がすべての管理機能(取り込み・削除・再起動)を使える。
- `sotaapp2.sotalive.net` は Cloudflare を通っている。

無料の案を比べた。

| 案 | 判断 |
|---|---|
| A. Cloudflare Access(Zero Trust Free、50 人まで。メールの使い捨ての番号・Google・GitHub で入れる。人を足すのはメールの許可の一覧だけ) | **採用** |
| B. Firebase のまま、役割の表を足す | 作る量は小さいが、アカウントとパスワードの管理が残る |
| C. GitHub OAuth を自分で作る | 作る量が大きい。GitHub を使わない人がいる |

役割は DB の表 `admin_users(email, role)` に持つ(admin / reviewer)。Access は「入れるか」、表は「何ができるか」を決める。

### 5. 今の管理者に迷惑をかけず、順に移す

利用者の要望で、移行を段階に分けた。

1. 管理の専用ホスト `admin.sotalive.net` を足し、Access でホスト全体を守る。バックエンドは Access の署名と Firebase のトークンのどちらでも通す(Firebase は今と同じく admin)。古い `/admin-console` は今のまま。
2. 2 人のメールを役割の表と Access に入れて、移ってもらう。どちらで入ったかをログで見る。
3. Firebase の利用が 2〜4 週間なくなったら、古い入り口を案内だけにして、Firebase を受け付けない。
4. `fireauth` などを消す。

ホスト全体を Access で守るので、管理の API を `/api/v2/admin` にまとめる手直しは要らなくなった。

利用者の答え:

- ホスト名は `admin.sotalive.net` でよい。
- Zero Trust は有効にした。

## 今の状態と次にやること

### 済んだこと(2026-10-25)

| 項目 | 場所 |
|---|---|
| 管理画面の実装(推定と目視を分けた表、見直しの API と画面、Cloudflare Access の認証、役割 admin / reviewer) | sotaapp2 PR #42 |
| `/pota` の速度の修正(公園の重複を除く副問い合わせが、呼ぶたびに `pota_references` の約 9.7 万行を走査していた) | sotaapp2 PR #43 |
| メールアドレスの伏せ字(管理画面とログ) | sotaapp2 PR #44 |
| バッチの出力を新しい形式にし、今の目視を移す道具(`summit_export`・`migrate_verdicts`・`verify_served`) | sotalive-monorepo PR #119 |
| 推定・地図・目視の移行・検証を 1 本で実行するスクリプト(`tools/jaffpota/scripts/load_summit_data.sh`) | sotalive-monorepo PR #120 |
| 通信を待たせない(短い時間切れと遮断 `lib/net.ts`)。地図は版が変わった地域だけ取り直す | sotalive-monorepo PR #121 |
| バッチの地図の作成を並列にする(`summit_export --jobs`。地図 6 分 → 39 秒、出力は直列とバイト同一) | sotalive-monorepo PR #122 |
| 公開の読み取り API に `Cache-Control` を付ける(スポット 30 秒、地磁気 10 分、検索 5 分、JCC/JCG 1 時間、地図の版 60 秒)。Cloudflare のキャッシュルールも設定済み | sotaapp2 PR #46、`docs/caching.md` |

本番(2026-10-25): Deploy 済み。推定 3585 行(2454 山頂、なしの組 132)、地図 7069 山頂、目視 69 山頂(177 組)を移行済み。
認証の移行は**段階 2 まで完了**(Access で入れる。管理者 2 人を `admin_users` に登録済み。もう一人の確認待ち)。Access のチームドメインは、Fly.io の secret `CF_ACCESS_TEAM_DOMAIN`、アプリの識別子は `CF_ACCESS_AUD`(値はリポジトリに書かない)。
ログイン方法は、One-time PIN(メールの番号)と Google の 2 つ(2026-10-25 に Google を足し、自分のアカウントで両方入れることを確認した)。Cloudflare のアカウントのログインは外した。

### 次にやること(候補)

1. **認証の段階 3**: ログの `method=Firebase` が 2〜4 週間なくなったら、古い `/admin-console` を新しい URL への案内だけにし、Firebase のトークンを受け付けない。段階 4 で `fireauth`・ログイン画面・Firebase の設定を消す。
2. (済み)FLEonline の地図の取り直し: 地図を開いたとき、版が変わった地域だけ取り直す(sotalive-monorepo PR #121)。
3. (済み)バッチの並列化(sotalive-monorepo PR #122)。残りは、増分の実行(前回の `input_rev` と比べて、変わった山頂だけを書き出す。運用が回り始めてから)。
4. **出していないもの**: `peak`(山頂の最高点。標高データが要る)、`change_note`(前の入力との差分)。上流(`batch_zones.py`・`review_data.py`・`triage.py`・`heur.py`)は、手元の `explore/` のまま。
5. **確定・取り消しの競合**: 確定の最中に地図が入れ直されると、tx の外で読んだ下地から配信用の地図が作られる(`reviewer` の指摘 4)。管理者が少人数なので後回しにした。
6. 案 C(公園ごとの区域の指紋)、報告の受け口(利用者が増えてから)。
7. 管理画面の実画面の確認(伏せ字、見直しのページ)。`npm run build` が追跡されている `static/` を書き換えるので、手元ではビルドしていない。

### 運用で分かったこと

- **Cloudflare の Cache Rules の「ブラウザー TTL」は「Respect origin TTL」にする。** 固定の時間(既定は 4 時間)を選ぶと、サーバーの `Cache-Control` が書き換わり、MyACT のブラウザーがスポットを数時間使い回す。エッジ TTL は「ヘッダーがなければバイパス」(ヘッダーのない `/sota/maps` や `/pota` を、既定の時間でキャッシュしないため)。詳しくは `docs/caching.md`。

- 本番は `shared-cpu-1x`(メモリ 256 MB)の 1 台。**クエリは全行の走査を避ける**(テーブルの規模は、ステージングでも本番と同じにして測る)。実行計画のテストで退行を防ぐ(`serving_query_looks_parks_up_by_index_not_by_scanning`)。
- **本番に対する検証は標本にする**(全山頂は約 1.4 万リクエスト。`verify_served --sample 300 --workers 2`)。全山頂の検証は、手元のステージングでだけ実行する。
- ステージングの DB の写しは、`cp` でなく `sqlite3 ".backup"` で取る(WAL の取りこぼしを避ける)。本番の写し(10/24 の保存)は、`summit_pota` を作るマイグレーションより前なので、旧の出力から移行前の状態を再現して使った。
- ログイン方法を Google と PIN の 2 つにしたので、Access のアプリの**インスタント認証はオフ**にする(方法が 2 つ以上あると使えない)。ログイン方法を外したり替えたりしたあとは、古いセッションが残るので、シークレットウィンドウか Cookie の削除で入り直す。Google のクライアント シークレットとチームドメインは、リポジトリに書かない。
- Access のチームドメインを変えると、ログイン済みのセッションの発行元(`iss`)が古いままになり、`InvalidIssuer` で弾かれる。シークレットウィンドウか、Cookie の削除で入り直す。画面に出る組織名は、チームドメインとは別の項目(設定 → カスタムページ)。

## 調べるときに見た場所

- sotaapp2 の今の実装: コミット `c362b3e`(summit_pota・summit_maps・取り込み・配信の API)。`api/src/handler/auth.rs`(`with_auth`)、`api/src/handler/sota.rs`(ルート)、`adapter/src/database/implement/sqlite/sota_reference.rs`。PostGIS の実装は更新されていない(sqlite だけで動かしている)。
- 管理画面: `admin/`(SvelteKit・Svelte 5・Tailwind。Docker でビルドするので `static/` はコミットしない)。
- FLEonline の地図: sotalive-monorepo の `apps/fle/src/lib/summitMap.ts`(GeoJSON の型・描き方・`PARK_STYLES`)、`apps/fle/src/components/MapSheet.tsx`。
- バッチと判定表: `~/sotaapp2-backups/sota_pota_20261024/`(`explore/`、`rereview/`、出力の最新は `sotaapp_out6/`)。
- 公園区域の版: sotalive-monorepo の `docs/park-areas.md`、`tools/jaffpota`。
