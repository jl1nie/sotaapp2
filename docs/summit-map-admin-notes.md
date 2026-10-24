# 山頂の地図と POTA の判定の管理画面: 検討の経緯

記録: 2026-10-25。続きの作業はここから始める。

- 元の計画: [summit-map-admin-plan.md](summit-map-admin-plan.md)
- 詳細設計(確認待ち): [summit-map-admin-design.md](summit-map-admin-design.md)
- ブランチ: `feat/summit-review`(`docs/summit-map-admin-plan` から切った。この 2 つの文書はまだコミットしていない)

## 進め方の決まり(利用者の指示)

- **設計は Opus、実装は Sonnet。** 詳細設計は Opus が書く。コードは Agent ツールで `model: "sonnet"` を指定したサブエージェントに書かせる。戻ってきた変更は Opus が読み、テストを通して確かめる。
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

1. **詳細設計の確認を待っている**(summit-map-admin-design.md)。最後に利用者へ確かめていたことが 2 つある。
   - 今の verified=1 の 98 行は、目視の移行(計画 7 章)まで推定として扱われ、配信の「目視済み」の印が消える。7 章が済むまで本番に出さない運用でよいか。
   - もう一人の管理者には、段階 2 で新しい URL(ログインはメールの番号)に移ってもらう。
2. 確認がとれたら、Sonnet のサブエージェント 2 つに並べて実装させる。
   - Rust: 認証・マイグレーション・repository・service・API・テスト
   - 管理画面: `admin/` の認証の流れ・見直しのページ・管理者の欄・vitest
   - どちらも sotaapp2 の `feat/summit-review` で作業し、ファイルは重ならない。コミットは Opus が確かめてから。
3. そのあとの PR:
   - FLEonline の地図の取り直し(`apps/fle/src/lib/mapStore.ts` の `REFRESH_MS` は今 30 日。地域の版の API を見て取り直す。monorepo)
   - バッチの変更(注釈・判定用の層・`input_rev`・版の列。`explore/export_sotaapp.py` と `review2_data.py` を `tools/jaffpota` に移す)と、目視の移行(計画 7 章)
   - 認証の移行の段階 3・4
   - 案 C(公園ごとの区域の指紋)、報告の受け口(利用者が増えてから)

## 調べるときに見た場所

- sotaapp2 の今の実装: コミット `c362b3e`(summit_pota・summit_maps・取り込み・配信の API)。`api/src/handler/auth.rs`(`with_auth`)、`api/src/handler/sota.rs`(ルート)、`adapter/src/database/implement/sqlite/sota_reference.rs`。PostGIS の実装は更新されていない(sqlite だけで動かしている)。
- 管理画面: `admin/`(SvelteKit・Svelte 5・Tailwind。Docker でビルドするので `static/` はコミットしない)。
- FLEonline の地図: sotalive-monorepo の `apps/fle/src/lib/summitMap.ts`(GeoJSON の型・描き方・`PARK_STYLES`)、`apps/fle/src/components/MapSheet.tsx`。
- バッチと判定表: `~/sotaapp2-backups/sota_pota_20261024/`(`explore/`、`rereview/`、出力の最新は `sotaapp_out6/`)。
- 公園区域の版: sotalive-monorepo の `docs/park-areas.md`、`tools/jaffpota`。
