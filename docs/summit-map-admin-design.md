# 山頂の簡易地図と POTA の判定の管理画面: 詳細設計

作成: 2026-10-25 / 元の計画: [summit-map-admin-plan.md](summit-map-admin-plan.md)(以下「計画」)

## 0. 目的と範囲

FLEonline の利用者から地図の誤り(DM で届く)を受けたとき、管理画面で答えを直し、直したものがすぐ利用者に届くようにする。直す人は、今の管理者 2 人と、のちに頼む数人。

この設計で作るもの(sotaapp2 の 1 つの PR):

1. **認証の移行の段階 1**(2 章): 管理の専用ホスト `admin.sotalive.net` を Cloudflare Access で守り、今の Firebase の入り口と**両方**使えるようにする。役割(admin / reviewer)を足す。
2. **目視の管理**(計画の 8 章の 1〜4): 推定と目視を分けた表・配信の答えの合成・配信用の地図の作り直し・見直しの API と画面。
3. **地図の版**: 配信用の地図を作り直した日時と、地域ごとの版を返す公開 API(計画 11 章の sotaapp2 側)。

後に回すもの: FLEonline の地図の取り直し(monorepo の別 PR)、バッチの変更と目視の移行(計画 6・7 章)、報告の受け口、Firebase の撤去(移行の段階 3・4)。

今のバッチの出力(`summit_pota.csv` の 6 列、答えを焼き込んだ地図)も、そのまま取り込めるようにする。注釈の列(7 章)は、バッチが出すようになったら画面に出る。

## 1. 移行の段階(運用)

| 段階 | 中身 | いつ |
|---|---|---|
| 1 | `admin.sotalive.net` を足す。認証は Firebase と Access のどちらでも通る(この PR) | このデプロイ |
| 2 | 2 人のメールを役割の表(admin)と Access の許可に入れ、新しい URL に移ってもらう。ログで Firebase の利用を見る | デプロイの後 |
| 3 | Firebase の利用が 2〜4 週間なくなったら、古い `/admin-console` を新しい URL への案内だけにし、Firebase のトークンを受け付けない | 別 PR |
| 4 | `fireauth`・ログインの画面・Firebase の設定を消す | 別 PR |

段階 1 で、Firebase で入った人は今と同じく admin として通す。今の管理者は、移るまで何も変わらない。

### Cloudflare・Fly.io の設定(利用者の操作。PR の説明に手順を書く)

1. DNS: `admin` の CNAME を `sotaapp2` と同じ先に(プロキシ有効)。
2. Fly.io: `fly certs add admin.sotalive.net`。
3. Zero Trust: Access のアプリ(Self-hosted)を `admin.sotalive.net` 全体に作る。ポリシーは Include: Emails(2 人)。ログインはメールの使い捨ての番号(One-time PIN)。アプリの **Application Audience (AUD) Tag** を控える。
4. Fly.io の secrets: `CF_ACCESS_TEAM_DOMAIN=https://<チーム名>.cloudflareaccess.com`、`CF_ACCESS_AUD=<AUD Tag>`。

FLEonline・MyACT が使う `sotaapp2.sotalive.net` には何もかけない。

取り込みの順は「推定の CSV を先に、地図を後に」。地図を先に入れると、`base_rev` が古い `input_rev` のままになり `map_mismatch` が出る。アップロードの上限は管理者で 8 MB なので、地図は `.gz` で送る。

## 2. 認証と役割

### 2.1 設定(`common/src/config.rs`)

- `cf_access_team_domain: Option<String>`、`cf_access_aud: Option<String>`(環境変数 `CF_ACCESS_TEAM_DOMAIN`・`CF_ACCESS_AUD`)。どちらかがなければ Access の検証はしない(Firebase だけ)。
- 手元の開発用: `ADMIN_DEV_EMAIL` があれば、その人として通す。**`#[cfg(debug_assertions)]` のときだけ**読む(Docker のリリースビルドでは効かない)。

### 2.2 Access の署名の検証(ポートと adapter)

- ポート: `domain/src/repository/access.rs` の `AccessTokenVerifier`(`is_enabled`・`verify`)。実装は `adapter/src/access.rs`(`AccessTokenVerifierImpl`)。`registry` で組み立て、`api` の `AdminAuth` はポートだけを見る。

- 署名は要求ヘッダー `Cf-Access-Jwt-Assertion`、なければ Cookie `CF_Authorization`。
- `jsonwebtoken` を workspace に足す。RS256。鍵は `{team_domain}/cdn-cgi/access/certs` の JWKS(`keys`)。
- 鍵は覚えておき、知らない `kid` が来たら取り直す(取り直しは 60 秒に 1 回まで、1 度に 1 つ。取り直しの HTTP を待つあいだも、知っている `kid` の検証は待たせない)。取れなければ 401。
- 確かめるもの: `aud` に `CF_ACCESS_AUD` が入っている、`iss == team_domain`、`exp`、`nbf`。`email` を取り出す(小文字にする)。
- Fly.io のアプリの URL へ直接来た要求は Cloudflare を通らないので、署名がない。この検証が守りの本体になる。

### 2.3 役割の表

```sql
CREATE TABLE admin_users (
    email      VARCHAR(255) NOT NULL PRIMARY KEY,   -- 小文字
    role       VARCHAR(16)  NOT NULL,               -- admin / reviewer
    created_at INTEGER      NOT NULL
);
```

マイグレーションでは行を入れない(メールを repo に置かない)。最初の 2 人は、段階 1 のあと、古い入り口(Firebase で入ると admin)の画面から足す(2.5)。

### 2.4 ミドルウェア(`api/src/handler/auth.rs`)

`with_auth` を次のように変える。

1. Access が設定されていて、署名があれば検証する。通れば `admin_users` でメールを引く。なければ 403(`FORBIDDEN`)。
2. 署名がなく、`Authorization: Bearer` があれば今の Firebase の検証。通れば role は admin。メールは Firebase の `email`。
3. どちらもなければ 401。

通ったら、extensions に次を入れる。

```rust
pub enum AuthMethod { Firebase, Access, Dev }
pub enum AdminRole { Admin, Reviewer }
pub struct AdminIdentity { pub email: String, pub role: AdminRole, pub method: AuthMethod }
```

- 今のハンドラーのために `UserId` も入れる。Firebase なら今と同じ `local_id`、Access なら `access:{email}`(使うのはログの取り込み・削除だけで、画面からは呼んでいない)。
- 要求ごとに `tracing::info!(method, email, path)` を出す(段階 2 で、Firebase がまだ使われているか見るため)。
- 役割で分ける: `with_auth(router, auth)` は admin だけ、`with_review_auth(router, auth)` は admin と reviewer。今の保護されたルートはすべて `with_auth`(admin だけ)のまま。見直しの API(6 章)だけが `with_review_auth`。

**CSRF**: Access の入り口は Cookie で通るので、ほかのサイトのフォームからの送信を断つ。Access で入った要求のうち GET・HEAD・OPTIONS 以外で、`Sec-Fetch-Site` が `same-origin`・`none` 以外なら 403(ヘッダーがなければ `Origin` と `Host` を比べる。どちらもなければ、ブラウザーでないので通す)。Firebase の Bearer は Cookie ではないので対象外。管理画面は `admin.sotalive.net` の同じ origin から呼ぶこと。

### 2.5 役割の API(admin だけ)

| メソッド | パス | 中身 |
|---|---|---|
| GET | `/admin/whoami` | `{ email, role, method }`(**reviewer も呼べる**。画面が自分の役割を知るため) |
| GET | `/admin/users` | `[{ email, role, createdAt }]` |
| PUT | `/admin/users/{email}` | 本文 `{ role }`。足す・変える |
| DELETE | `/admin/users/{email}` | 消す。自分(Access で入っている人)自身と、最後の admin は消せない(400)。最後の admin を reviewer に下げることもできない(400) |

## 3. マイグレーション

`adapter/migrations/sqlite/20261025120000_summit_review.up.sql`(PostGIS には summit_pota もないので作らない)。

- `admin_users`(2.3)。
- 計画 3.1 の表 5 つ。計画との違い:
  - **人はメールで持つ**: `summit_reviews.reviewed_by`・`summit_pota_log.changed_by` はメール(UID ではない)。
  - **版を分ける**: `summit_estimates_meta.data_version` の代わりに `park_version`・`muni_version`・`munitable_version`(どれも `VARCHAR(64) NOT NULL`)。`summit_reviews` にも同じ 3 列を持ち、確定したときの版を写す。
  - **位置は NULL を許す**: `summit_estimates_meta.latitude`・`longitude`。
  - **注釈の列**(判定表 `rereview/data3.json` と同じもの。バッチが出すまで NULL): `summit_pota_estimates` に `tot_px INTEGER`、`dist_m REAL`、`shallow INTEGER NOT NULL DEFAULT 0`、`hint VARCHAR(16)`、`needs_review INTEGER NOT NULL DEFAULT 0`。`summit_estimates_meta` に `reasons TEXT`(`muni,near,shallow` の並び)と `change_note TEXT`(前の入力との差分の要約。計画 10.3)。
  - `CREATE INDEX idx_summit_pota_log_summit ON summit_pota_log(summit_code, id)`。
- 移し替え:
  - `summit_pota` の全行を `summit_pota_estimates` に入れる。verified=1 の行も推定として入れる(目視は 7 章の移行で入れ直す。それまで配信の `verified` は false)。
  - `summit_pota` と `summit_maps` に出てくる山頂ごとに、`summit_estimates_meta` を 1 行入れる。`input_rev = 'legacy'`、版の 3 列は `'legacy'`、位置は `sota_references` から(なければ NULL)、`computed_at = strftime('%s','now')`。
  - `DROP TABLE summit_pota`。
- 地図:
  - `ALTER TABLE summit_maps ADD COLUMN base_geojson TEXT`、`ADD COLUMN base_rev VARCHAR(64)`、`ADD COLUMN served_at INTEGER`。
  - `UPDATE summit_maps SET base_geojson = geojson, base_rev = 'legacy', served_at = strftime('%s','now')`。
  - `geojson` は**配信用**、`base_geojson` は**下地**。`base_rev` は下地を入れたときの推定の `input_rev`(meta と違えば、推定と地図の版がずれている)。`served_at` は配信用を作り直した日時。

## 4. ドメイン(`domain/src/model/sota.rs`)

- `ParkCoverage` に `None`("none")を足す。配信(`SummitPota`)には All / Part しか入らない。
- 足す型(derive は `Debug, Clone, PartialEq`):

```rust
pub struct DataVersions { park: String, muni: String, munitable: String }

pub struct SummitPotaEstimate { summit_code, muni_key, muni_name, pota_code: String, coverage: ParkCoverage,
    pct: Option<f64>, px: Option<i64>, tot_px: Option<i64>, dist_m: Option<f64>, noise: bool, shallow: bool,
    hint: Option<ParkCoverage>, needs_review: bool, note: Option<String> }
pub struct SummitEstimateMeta { summit_code, input_rev: String, latitude: Option<f64>, longitude: Option<f64>,
    versions: DataVersions, reasons: Vec<String>, change_note: Option<String>, computed_at: DateTime<Utc> }
pub struct SummitPotaVerdict { muni_key, pota_code: String, coverage: ParkCoverage }
pub struct SummitReview { summit_code, input_rev, reviewed_by: String /* メール */, versions: DataVersions,
    reviewed_at: DateTime<Utc>, note: Option<String> }
pub struct SummitPotaLogEntry { muni_key, pota_code: String, before: Option<ParkCoverage>, after: Option<ParkCoverage>,
    changed_by: String, changed_at: DateTime<Utc> }

/// 配信用の地図を作るための、組ごとの答え(目視があれば目視、なければ推定)。無効・未登録の公園の組は入れない
pub struct ComposedPair { muni_key, pota_code: String, park_name: Option<String>, coverage: ParkCoverage }
pub struct SummitMapInput { summit_code, base_geojson: String, pairs: Vec<ComposedPair> }

/// 一覧の 1 行(状態は service で計算する)
pub struct SummitReviewSummary {
    summit_code, summit_name: String,           // summit_name_j があればそれ、なければ summit_name
    ref_latitude: Option<f64>, ref_longitude: Option<f64>,
    meta: Option<SummitEstimateMeta>, has_map: bool, base_rev: Option<String>,
    pairs: i64, all_pairs: i64, part_pairs: i64, none_pairs: i64,
    flagged_pairs: i64,                          // needs_review・noise・shallow のどれか、または note がある組
    review: Option<SummitReview>,
}

/// 詳細(管理画面の 1 山頂)
pub struct SummitPairDetail { estimate: SummitPotaEstimate, park_name: Option<String>,
    park_inactive: bool /* park_inactive または pota_references にない */ }
pub struct SummitReviewDetail {
    summit_code, summit_name: String, alt_m: i32, latitude: Option<f64>, longitude: Option<f64>,
    city_j: Option<String>, activation_count: i64,
    meta: Option<SummitEstimateMeta>,
    pairs: Vec<SummitPairDetail>,
    verdicts: Vec<SummitPotaVerdict>,            // 今の候補にない組の目視も含めて全部
    review: Option<SummitReview>,
    base_geojson: Option<String>, base_rev: Option<String>, served_at: Option<DateTime<Utc>>,
    log: Vec<SummitPotaLogEntry>,                // 新しい順、200 件まで
}
```

## 5. 状態の計算(service の純粋な関数。単体テストを書く)

```rust
pub enum ReviewStatus { NoMap, Unreviewed, NeedsCheck, Rereview, Confirmed }   // serde は snake_case
```

- 今の `input_rev` は `meta.input_rev`(meta がなければ `""`)。
- `NoMap`: `has_map == false`。
- それ以外で review があれば、`review.input_rev == 今の input_rev` なら `Confirmed`、違えば `Rereview`。
- review がなく、`part_pairs > 0 || flagged_pairs > 0 || !meta.reasons.is_empty()` なら `NeedsCheck`、でなければ `Unreviewed`(候補の組が 0 の山頂も、空の答えで確定できる)。
- 状態とは別のフラグ:
  - `stale`(推定が古い): meta と sota_references の両方に位置があり、距離が 20 m 以上。
  - `map_mismatch`(地図と推定の版がずれている): `base_rev` があり、`'legacy'` でなく、今の `input_rev` と違う。

## 6. Repository(`SotaRepository` と sqlite の実装)

`replace_summit_pota` は消す。足す・変えるもの:

| メソッド | 中身 |
|---|---|
| `import_summit_estimates(estimates, metas, full: bool) -> AppResult<usize>` | 1 つの tx。`metas` の山頂ごとに推定を消して入れ直し、meta を upsert。`full` なら、`metas` にない山頂の推定・meta を消す(目視・review・log・地図は消さない。候補の組が 0 の山頂は CSV に行を書けず、地図だけを持つため)。返り値は、確定だった(review.input_rev == 前の meta.input_rev)のに input_rev が変わった山頂の数(=要再確認になった数) |
| `find_summit_pota(code)` | 配信の答え。下の SQL |
| `find_composed_pairs(codes: &[String]) -> HashMap<String, Vec<ComposedPair>>` | 地図の取り込みのとき用 |
| `find_summit_map_inputs(codes: &[String]) -> Vec<SummitMapInput>` | 下地と答え。推定の取り込み後の作り直し用 |
| `upsert_summit_maps(maps: Vec<(SummitMap /*下地*/, String /*配信用*/)>, now)` | 下地・配信用を upsert。`base_rev` はその山頂の meta の input_rev(なければ NULL)、`served_at = now` |
| `update_served_maps(maps: Vec<SummitMap>, now)` | 配信用(`geojson`)と `served_at` だけを更新 |
| `find_summit_map(code)` / `find_summit_maps(prefix)` | `sota_references` に JOIN して、ない山頂は返さない |
| `find_map_versions(prefix) -> Vec<(String /*地域 "JA/KN"*/, DateTime<Utc>)>` | 地域ごとの `MAX(served_at)`。地域は山頂コードの `-` の前 |
| `find_summit_review_list(prefix) -> Vec<SummitReviewSummary>` | `sota_references` の prefix の範囲(今の `find_summit_maps` と同じ範囲の書き方)を基に、meta・reviews を LEFT JOIN。推定は集計のサブクエリ、`has_map` は EXISTS |
| `find_summit_review_detail(code) -> Option<SummitReviewDetail>` | `sota_references` にも推定にも地図にもなければ None |
| `save_summit_review(SaveSummitReview) -> AppResult<()>` | 下記 |
| `delete_summit_review(DeleteSummitReview) -> AppResult<()>` | 下記 |
| `list_admin_users` / `find_admin_user(email)` / `upsert_admin_user` / `delete_admin_user` | 2.3 の表。置き場所は新しい `AdminUserRepository`(domain の trait と sqlite の実装、registry に登録) |

配信の答え(`find_summit_pota`。`find_composed_pairs` も同じ規則):

```sql
SELECT e.summit_code, e.muni_key, e.muni_name, e.pota_code, p.park_name_j AS park_name,
       COALESCE(v.coverage, e.coverage) AS coverage, (v.coverage IS NOT NULL) AS verified
FROM summit_pota_estimates AS e
JOIN sota_references AS s ON s.summit_code = e.summit_code
JOIN pota_references AS p ON p.pota_code = e.pota_code AND p.park_inactive = 0
LEFT JOIN summit_pota_verdicts AS v
  ON v.summit_code = e.summit_code AND v.muni_key = e.muni_key AND v.pota_code = e.pota_code
WHERE e.summit_code = $1 AND COALESCE(v.coverage, e.coverage) IN ('all', 'part')
ORDER BY e.muni_key, e.pota_code
```

推定にない組の目視は配信しない。`find_composed_pairs` は none も返してよいが、地図の差し込みでは all / part だけを使う。

`SaveSummitReview { summit_code, expected_input_rev, answers: Vec<SummitPotaVerdict>, reviewed_by, note, now, served_geojson: Option<String> }` を、1 つの tx で次の順に行う。

1. 今の meta の input_rev(なければ `""`)が `expected_input_rev` と違えば `AppError::Conflict`。
2. 前の verdicts を読み、組ごとに前と後が違うものを log に積む(前にあって後にない組は after = NULL)。
3. その山頂の verdicts を置き換える。
4. review を upsert する(input_rev は expected_input_rev、版の 3 列は今の meta から写す)。
5. `served_geojson` があれば、`summit_maps.geojson` と `served_at` を更新する。

`DeleteSummitReview { summit_code, changed_by, now, served_geojson }`: 1 つの tx で、verdicts を log に積んで(after = NULL)消し、review を消し、配信用の地図を更新する。

## 7. 地図の差し込み(service の純粋な関数。単体テストを書く)

`fn build_served_map(base: &str, pairs: &[ComposedPair]) -> AppResult<String>`

- 下地を `serde_json::Value` で読む。上の階層(type・summit・bbox・attribution など)はそのまま。
- `kind` が `judge_park`・`peak` の feature は外す(判定用なので端末には送らない)。
- `kind == "muni"`: `properties.pota` を、その `key` の組のうち all / part のものから作り直す。形は `[{code, name, coverage}]` で、code の順。park_name がないときは `name` を入れない。
- `kind == "park"`: (`code`, `key`) の組の答えが part のときだけ残す。
- ほかの kind はそのまま。出力はつめた JSON(1 行)。
- 今のバッチの地図(muni に答えが焼き込まれ、park は「一部」の分だけ)にも、そのまま使える。

作り直すのは書いたときだけ(計画 3.4):

- 地図の取り込み: 行を全部確かめたあと、500 件ずつ `find_composed_pairs` → 差し込み → `upsert_summit_maps`。
- 推定の取り込み: 取り込んだ山頂を 500 件ずつ `find_summit_map_inputs` → 差し込み → `update_served_maps`。塊の間で `tokio::task::yield_now()`。
- 目視の確定・取り消し: その山頂だけ(tx の中)。

## 8. 推定の CSV の取り込み

`POST /sota/import/pota?mode=full`。mode を省くと、ファイルにある山頂だけを置き換える。

- **見出しの名前で読む**(`csv::ReaderBuilder` を has_headers で使う。今の `csv_reader` は位置で読むので使わない)。
  - 必須: `summit_code, muni_key, muni_name, pota_code, coverage`(all / part / none)。
  - 任意(組の列): `pct, px, tot_px, dist_m, noise, shallow, hint, needs_review, note`。
  - 任意(山頂の列。同じ山頂の行で同じ値): `input_rev, latitude, longitude, park_version, muni_version, munitable_version, reasons, change_note`。
  - `verified` は読み捨てる(今のバッチの CSV にある列)。空の欄は None。真偽は `0/1`。
- 同じ山頂の行で山頂の列が食い違えば 422。同じ組が 2 回出ても 422。
- `input_rev` がない山頂は、組 `muni_key|pota_code` を並べて改行でつないだものの SHA-256 の先頭 16 桁に `pairs:` を付けたものにする(組が変われば要再確認になる)。workspace に `sha2` を足す。
- 位置がなければ、repository で `sota_references` から入れる。版の列がなければ `"unknown"`。
- 返り値は `{ success, message, imported /*行*/, summits, rereview }`。message は「N 行(M 山頂)を取り込みました。要再確認になった山頂: K」。

## 9. API

山頂コードは `/` を含むので、管理画面はパスに `encodeURIComponent` して送る(`JA%2FKN-017`。axum の `Path` で元に戻る)。

### 9.1 見直し(`with_review_auth`。`api/src/handler/summit_review.rs` を新しく作り、`build_sota_routers` に入れる)

| メソッド | パス | 本文・返り値 |
|---|---|---|
| GET | `/sota/review/summits?prefix=&status=&q=` | `SummitReviewListView` |
| GET | `/sota/review/summits/{code}` | `SummitReviewDetailView`。なければ 404 |
| PUT | `/sota/review/summits/{code}` | 本文 `{ inputRev, answers: [{muniKey, potaCode, coverage}], note? }` → `SummitReviewDetailView` |
| DELETE | `/sota/review/summits/{code}` | → `SummitReviewDetailView` |

- `prefix` は必須。`JA`(協会)か `JA/KN`(地域)。今の `region_prefix` を広げ、協会だけなら `JA/` にする。読めなければ 400。
- `status`: `no_map | unreviewed | needs_check | rereview | confirmed | stale | map_mismatch`。
- `q`: 山頂コード・山名の部分一致(大文字小文字を区別しない)。
- `counts` は prefix の範囲全体の件数(status・q で絞る前)。

```ts
SummitReviewListView = {
  counts: { noMap, unreviewed, needsCheck, rereview, confirmed, stale, mapMismatch: number },
  summits: { summitCode, summitName, pairs, allPairs, partPairs, nonePairs, flaggedPairs,
             status, stale: boolean, mapMismatch: boolean,
             reviewedBy?: string, reviewedAt?: string /*ISO*/ }[]           // 山頂コードの順
}
type Versions = { park, muni, munitable: string }
SummitReviewDetailView = {
  summitCode, summitName, altM, latitude?, longitude?, cityJ?, activationCount,
  status, stale, mapMismatch, inputRev, versions?: Versions, reasons: string[], changeNote?, computedAt?,
  pairs: { muniKey, muniName, inCityJ: boolean /* city_j に <muniKey> がある */,
           potaCode, parkName?, parkInactive: boolean,
           estimate: 'all'|'part'|'none', pct?, px?, totPx?, distM?, noise, shallow, hint?, needsReview, note?,
           verdict?: 'all'|'part'|'none' }[],                                // muni_key, pota_code の順
  previousVerdicts: { muniKey, potaCode, coverage }[],                       // 今の候補にない組の目視(参考)
  review?: { inputRev, reviewedBy, reviewedAt, versions: Versions, note? },
  baseMap?: object,                                                          // 下地の GeoJSON(文字列ではなく JSON で)
  baseRev?, servedAt?,
  log: { muniKey, potaCode, before?, after?, changedBy, changedAt }[]
}
```

PUT の検証(service):

1. 詳細を読む。なければ 404。
2. 答えの組が今の推定の組と**ぴったり一致**しなければ `AppError::BadRequest`(400)。足りない・余計・重複のどれもこれに当たる。
3. `inputRev` が今と違えば `AppError::Conflict`(409)。
4. 答えで配信用の地図を作る(下地があれば)。
5. `save_summit_review`(reviewed_by は `AdminIdentity.email`)。
6. 保存後の詳細を返す。

`AppError` に `BadRequest(String)`(400, `BAD_REQUEST`)、`Conflict(String)`(409, `CONFLICT`)、`Forbidden(String)`(403, `FORBIDDEN`。今の権限の variant が使えればそれでよい)を足す。

### 9.2 地図の版(公開。認証なし)

`GET /sota/maps/versions?prefix=JA` → `[{ region: "JA/KN", updatedAt: "ISO" }]`。prefix は協会か地域。FLEonline はこれを見て、変わった地域だけ取り直す(monorepo の別 PR)。

### 9.3 OpenAPI

`sota.rs` の今のハンドラーには `utoipa::path` が付いていないので、合わせて付けない。

## 10. 管理画面(`admin/`)

### 10.1 認証

- `auth.ts`: 起動時に `GET /api/v2/admin/whoami` を呼ぶ。
  - トークンなしで 200 なら Access(か Dev)で入っている。ログインの画面を出さず、要求に `Authorization` を付けない(Cookie で通る)。
  - 401 なら、今の Firebase の流れ(localStorage のトークン → なければログインの画面)。
- 状態に `email`・`role`・`method` を持つ。
- Access の期限が切れると、fetch は Access のログインへの転送(HTML)を受け取る。JSON でない応答や `redirected` を見たら、ページを読み直す。
- reviewer の人には、ダッシュボードを出さず、見直しのページ(10.2)だけを出す。admin には、ダッシュボードに「管理者」の欄(2.5 の一覧・追加・役割の変更・削除)を足す。

### 10.2 見直しのページ

- `src/routes/admin-console/summit-maps/+page.svelte`。1 ページで一覧と詳細を出す。`?prefix=JA/KN&status=&q=&code=JA/KN-017` を URL に持ち、読み直しても同じ所に戻る。
- ダッシュボード(`AdminDashboard.svelte`)から「SOTA × POTA」「Summit Maps」のアップロード欄を外し、このページへのリンクを置く。2 つのアップロード欄はこのページの上に置き、admin のときだけ出す。推定の取り込みには「全件(ファイルにない山頂を消す)」のチェックを付ける(mode=full)。
- 書き方は今の `sota/+page.svelte` に合わせる(認証のガード、Tailwind の slate の暗い配色、`$lib/api.ts` の fetch の書き方)。
- `src/lib/summit-review/` に置くもの:
  - `api.ts`: 9 章の型と呼び出し。409 は `{ conflict: true }` のように区別して返す。
  - `mapDraw.ts`: FLEonline の `apps/fle/src/lib/summitMap.ts`(sotalive-monorepo)から `projector`・`pathOf`・`PARK_STYLES`・`parkCodesOf`・`parkStyleOf`・`labelPointIn`・`withoutPrefecture` を写す。先頭に出どころのコメントを書く。
  - `logic.ts`: 純粋な関数。`initialChoices(detail)`、`canConfirm(pairs, choices)`、`statusLine(...)`、`buildServedMap(baseMap, pairs, choices)`(7 章と同じ規則を TS で書く)。
  - `JudgeMap.svelte`: 判定用の地図。ゾーン(赤)・山頂(▲)・最高点(○、`peak` があれば)・市区町村境(破線)・公園の元の区域(`judge_park` があればそれ、なければ `park`。公園ごとの色)・100 m の目盛り。
  - `FinalMap.svelte`: `buildServedMap` の結果を、FLEonline の `MapSheet.tsx` と同じ見え方で描く(公園ごとの斜線の色と向き・区域の番号・ゾーン・市区町村境・山頂・市区町村ごとの POTA の文字)。選ぶたびに描き直す。
- 詳細の決まり(計画 2 章・5 章。判定表 `rereview/index.html` のカードを引き継ぐ):
  - **初期値**:
    - `status == confirmed` なら、目視(`verdict`)を初期値にする。
    - `rereview` では初期値を入れない。前の答えは「前の目視」の列に参考として出す(`previousVerdicts` も)。
    - それ以外も初期値なし。推定は初期値にしない。
  - 山頂の見出し: コード・山名・標高・運用の回数・city_j・理由のチップ(`reasons`)・データの版・`changeNote`。`stale`・`mapMismatch` なら注意を出す。
  - 組ごとの行:
    - 市区町村(city_j にないなら印)と、公園(コード・名前。無効なら印)。
    - 推定の答えと根拠。「ゾーンのこの部分の pct% が公園」、px が 0 なら「公園まで distM m」、noise・shallow の注記、参考(`hint`)、note。
    - ラジオ(全部 / 一部 / なし)。`needsReview` の組は、選ぶまで色を付ける。
    - 注釈がない(今のバッチの)組は、推定の答えだけを出す。
  - 状態の行は次を出し分ける。「あと N 組を選んでください」「選びました(まだ確定していません)」「保存しています…」「確定 日時・メール」「保存できませんでした: 理由」。
  - **確定のボタン**:
    - 全組を選ぶまで押せない。
    - 押すと「保存中」になり、画面全体を押せなくする。返事の詳細で画面を置き換えて「確定」にする。
    - 失敗したら、選んでいたものを残して理由を出す。409 なら「見ている間に推定が取り込み直されました。読み直してください」と、読み直しのボタンを出す。
  - 確定しても画面は動かない。進むのは「前の山頂」「次の山頂」(一覧の絞り込みの順)だけ。選び直しが確定していないまま移るときは、confirm で確かめる。
  - メモ欄(`note`)、「目視を取り消す」(confirm で確かめてから DELETE)、履歴の表を置く。
  - 外部の地図: MyACT `https://myact.sotalive.net/?lat=&lon=&z=15`、地理院地図 `https://maps.gsi.go.jp/#16/{lat}/{lon}/`。
- テスト:
  - `vitest` を devDependencies に足し、`logic.ts` の単体テストを書く(初期値が選ばれない・rereview で初期値なし・全組を選ぶまで確定できない・差し込みの規則)。
  - `npm run check` と `npm run build` が通ること。
  - ビルドの出力(`static/`)はコミットしない(Docker でビルドする)。

## 11. Rust のテスト(計画 9 章に足す)

- 認証:
  - JWT の検証(テスト用の RSA の鍵で署名したもの。aud・iss・exp が違うと通らない、知らない kid で取り直す)。
  - ミドルウェアの役割の分け方(reviewer は見直しの API だけ通る、admin_users にないメールは 403、Firebase は admin)。
- repository(一時 DB にマイグレーションをかける、今のテストの書き方):
  - 取り込みで目視が変わらない。
  - 候補が減っても目視は残り、要再確認の数が返る。
  - 山頂を絞った取り込みで、ファイルにない山頂が変わらない。full でも目視は消えない。
  - 配信で目視が推定より優先され、none は外れる。
  - 無効な公園・sota_references にない山頂は配信しない。
  - save の 409・log の積み方。delete。
  - 地域ごとの版。admin_users。
- service: 状態の計算(stale・map_mismatch を含む)、地図の差し込み、PUT の 400 / 409、CSV の読み方(今の 6 列の CSV・新しい列・食い違い)。
- `cargo make clippy` と `cargo make test` が通ること。`.sqlx` は一時 DB で作り直す。
