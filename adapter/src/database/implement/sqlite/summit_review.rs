//! 山頂の地図と POTA の判定(推定・目視・配信)の SQL。`SotaRepositoryImpl` から呼ぶ。
//! 設計: docs/summit-map-admin-design.md 6 章

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use common::error::{db_error, tx_error, AppError, AppResult};
use domain::model::sota::{
    ComposedPair, DataVersions, DeleteSummitReview, ParkCoverage, SaveSummitReview, SummitCode,
    SummitEstimateMeta, SummitMap, SummitMapInput, SummitPairDetail, SummitPota,
    SummitPotaEstimate, SummitPotaLogEntry, SummitPotaVerdict, SummitReview, SummitReviewDetail,
    SummitReviewSummary,
};
use sqlx::sqlite::{SqlitePool, SqliteRow};
use sqlx::{QueryBuilder, Row, Sqlite, SqliteConnection};

/// SQLite の変数の上限(32766)に収まる、IN (...) の 1 回あたりの件数
const IN_CHUNK: usize = 500;

/// 公園の名前と有効かどうかを、`e.pota_code` で索引(idx_pota_references_code)を引いて求める。
/// pota_references の主キーは wwff_code も含むので、同じ pota_code の行が重複しうる。名前は MIN、
/// 有効は「1 行でも有効なら有効」(MIN(park_inactive) = 0)。行がなければ NULL(= 有効でない)。
///
/// pota_references は約 9.7 万行ある。`GROUP BY pota_code` の副問い合わせを JOIN すると、呼ぶたびに全行を走査し、
/// 一時的な索引まで作る(本番の小さなマシンで、/pota が数秒かかった)。必ずこの相関サブクエリを使う。
const PARK_NAME: &str =
    "(SELECT MIN(p.park_name_j) FROM pota_references AS p WHERE p.pota_code = e.pota_code)";
const PARK_INACTIVE: &str =
    "(SELECT MIN(p.park_inactive) FROM pota_references AS p WHERE p.pota_code = e.pota_code)";

fn at(ts: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(ts, 0).unwrap_or_default()
}

fn cov(s: &str) -> ParkCoverage {
    ParkCoverage::parse(s).unwrap_or(ParkCoverage::None)
}

fn cov_opt(s: Option<String>) -> Option<ParkCoverage> {
    s.as_deref().and_then(ParkCoverage::parse)
}

/// 山頂コードが prefix で始まる範囲(LIKE の % と _ を避けるため範囲で絞る)
fn prefix_range(prefix: &str) -> (String, String) {
    (prefix.to_string(), format!("{}\u{10FFFF}", prefix))
}

fn meta_from(row: &SqliteRow, p: &str) -> Option<SummitEstimateMeta> {
    let input_rev: Option<String> = row.get(format!("{p}input_rev").as_str());
    let input_rev = input_rev?;
    let reasons: Option<String> = row.get(format!("{p}reasons").as_str());
    Some(SummitEstimateMeta {
        summit_code: row.get("summit_code"),
        input_rev,
        latitude: row.get(format!("{p}latitude").as_str()),
        longitude: row.get(format!("{p}longitude").as_str()),
        versions: DataVersions {
            park: row.get(format!("{p}park_version").as_str()),
            muni: row.get(format!("{p}muni_version").as_str()),
            munitable: row.get(format!("{p}munitable_version").as_str()),
        },
        reasons: reasons
            .map(|r| {
                r.split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        change_note: row.get(format!("{p}change_note").as_str()),
        computed_at: at(row.get(format!("{p}computed_at").as_str())),
    })
}

fn review_from(row: &SqliteRow) -> Option<SummitReview> {
    let input_rev: Option<String> = row.get("r_input_rev");
    Some(SummitReview {
        summit_code: row.get("summit_code"),
        input_rev: input_rev?,
        reviewed_by: row.get("r_reviewed_by"),
        versions: DataVersions {
            park: row.get("r_park_version"),
            muni: row.get("r_muni_version"),
            munitable: row.get("r_munitable_version"),
        },
        reviewed_at: at(row.get("r_reviewed_at")),
        note: row.get("r_note"),
    })
}

// ---- 取り込み ----

pub async fn import_summit_estimates(
    pool: &SqlitePool,
    estimates: Vec<SummitPotaEstimate>,
    metas: Vec<SummitEstimateMeta>,
    full: bool,
) -> AppResult<usize> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(tx_error("begin import_summit_estimates"))?;
    let mut rereview = 0usize;

    for m in &metas {
        let prev: Option<String> = sqlx::query_scalar(
            "SELECT input_rev FROM summit_estimates_meta WHERE summit_code = $1",
        )
        .bind(&m.summit_code)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db_error("select prev meta"))?;
        let reviewed: Option<String> =
            sqlx::query_scalar("SELECT input_rev FROM summit_reviews WHERE summit_code = $1")
                .bind(&m.summit_code)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_error("select review rev"))?;
        if let (Some(p), Some(r)) = (&prev, &reviewed) {
            if p == r && *p != m.input_rev {
                rereview += 1;
            }
        }
        sqlx::query("DELETE FROM summit_pota_estimates WHERE summit_code = $1")
            .bind(&m.summit_code)
            .execute(&mut *tx)
            .await
            .map_err(db_error("delete estimates"))?;
        sqlx::query(
            r#"
            INSERT INTO summit_estimates_meta(summit_code, input_rev, latitude, longitude,
                park_version, muni_version, munitable_version, reasons, change_note, computed_at)
            VALUES($1, $2,
                   COALESCE($3, (SELECT latitude FROM sota_references WHERE summit_code = $1)),
                   COALESCE($4, (SELECT longitude FROM sota_references WHERE summit_code = $1)),
                   $5, $6, $7, $8, $9, $10)
            ON CONFLICT (summit_code) DO UPDATE SET
                input_rev = EXCLUDED.input_rev, latitude = EXCLUDED.latitude,
                longitude = EXCLUDED.longitude, park_version = EXCLUDED.park_version,
                muni_version = EXCLUDED.muni_version, munitable_version = EXCLUDED.munitable_version,
                reasons = EXCLUDED.reasons, change_note = EXCLUDED.change_note,
                computed_at = EXCLUDED.computed_at
        "#,
        )
        .bind(&m.summit_code)
        .bind(&m.input_rev)
        .bind(m.latitude)
        .bind(m.longitude)
        .bind(&m.versions.park)
        .bind(&m.versions.muni)
        .bind(&m.versions.munitable)
        .bind(m.reasons.join(","))
        .bind(&m.change_note)
        .bind(m.computed_at.timestamp())
        .execute(&mut *tx)
        .await
        .map_err(db_error("upsert meta"))?;
    }

    for e in &estimates {
        sqlx::query(
            r#"
            INSERT INTO summit_pota_estimates(summit_code, muni_key, muni_name, pota_code, coverage,
                pct, px, tot_px, dist_m, noise, shallow, hint, needs_review, note)
            VALUES($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
        )
        .bind(&e.summit_code)
        .bind(&e.muni_key)
        .bind(&e.muni_name)
        .bind(&e.pota_code)
        .bind(e.coverage.as_str())
        .bind(e.pct)
        .bind(e.px)
        .bind(e.tot_px)
        .bind(e.dist_m)
        .bind(e.noise as i64)
        .bind(e.shallow as i64)
        .bind(e.hint.map(|h| h.as_str()))
        .bind(e.needs_review as i64)
        .bind(&e.note)
        .execute(&mut *tx)
        .await
        .map_err(db_error("insert estimate"))?;
    }

    if full {
        // ファイルにない山頂の推定・meta を消す(目視・確定・履歴・地図は消さない)。
        // 候補の組が 0 の山頂は CSV に行を書けないので、ファイルにないことは地図を消す理由にならない
        sqlx::query("CREATE TEMP TABLE IF NOT EXISTS keep_summits(summit_code TEXT PRIMARY KEY)")
            .execute(&mut *tx)
            .await
            .map_err(db_error("create keep_summits"))?;
        sqlx::query("DELETE FROM keep_summits")
            .execute(&mut *tx)
            .await
            .map_err(db_error("clear keep_summits"))?;
        for m in &metas {
            sqlx::query("INSERT OR IGNORE INTO keep_summits(summit_code) VALUES($1)")
                .bind(&m.summit_code)
                .execute(&mut *tx)
                .await
                .map_err(db_error("insert keep_summits"))?;
        }
        for table in ["summit_pota_estimates", "summit_estimates_meta"] {
            sqlx::query(&format!(
                "DELETE FROM {table} WHERE summit_code NOT IN (SELECT summit_code FROM keep_summits)"
            ))
            .execute(&mut *tx)
            .await
            .map_err(db_error("delete summits not in file"))?;
        }
        sqlx::query("DROP TABLE keep_summits")
            .execute(&mut *tx)
            .await
            .map_err(db_error("drop keep_summits"))?;
    }

    tx.commit()
        .await
        .map_err(tx_error("commit import_summit_estimates"))?;
    Ok(rereview)
}

// ---- 配信の答え ----

/// 配信の答え(目視があれば目視、なければ推定。all/part で、有効な公園の組だけ)を引く SQL
fn served_sql() -> String {
    format!(
        r#"
        SELECT e.summit_code, e.muni_key, e.muni_name, e.pota_code, {PARK_NAME} AS park_name,
               COALESCE(v.coverage, e.coverage) AS coverage, (v.coverage IS NOT NULL) AS verified
        FROM summit_pota_estimates AS e
        JOIN sota_references AS s ON s.summit_code = e.summit_code
        LEFT JOIN summit_pota_verdicts AS v
          ON v.summit_code = e.summit_code AND v.muni_key = e.muni_key AND v.pota_code = e.pota_code
        WHERE e.summit_code = $1 AND COALESCE(v.coverage, e.coverage) IN ('all', 'part')
          AND {PARK_INACTIVE} = 0
        ORDER BY e.muni_key, e.pota_code
    "#
    )
}

pub async fn find_summit_pota(
    pool: &SqlitePool,
    summit_code: &SummitCode,
) -> AppResult<Vec<SummitPota>> {
    let sql = served_sql();
    let rows = sqlx::query(&sql)
        .bind(summit_code.inner_ref())
        .fetch_all(pool)
        .await
        .map_err(db_error("select summit_pota"))?;
    Ok(rows
        .iter()
        .map(|r| SummitPota {
            summit_code: r.get("summit_code"),
            muni_key: r.get("muni_key"),
            muni_name: r.get("muni_name"),
            pota_code: r.get("pota_code"),
            park_name: r.get("park_name"),
            coverage: cov(&r.get::<String, _>("coverage")),
            verified: r.get::<i64, _>("verified") != 0,
        })
        .collect())
}

/// 無効・未登録の公園の組を除いた、組ごとの答え(目視があれば目視、なければ推定。none も含む)
pub async fn find_composed_pairs(
    pool: &SqlitePool,
    codes: &[String],
) -> AppResult<HashMap<String, Vec<ComposedPair>>> {
    let mut out: HashMap<String, Vec<ComposedPair>> = HashMap::new();
    for chunk in codes.chunks(IN_CHUNK) {
        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(format!(
            r#"
            SELECT e.summit_code, e.muni_key, e.pota_code, {PARK_NAME} AS park_name,
                   COALESCE(v.coverage, e.coverage) AS coverage
            FROM summit_pota_estimates AS e
            LEFT JOIN summit_pota_verdicts AS v
              ON v.summit_code = e.summit_code AND v.muni_key = e.muni_key AND v.pota_code = e.pota_code
            WHERE {PARK_INACTIVE} = 0 AND e.summit_code IN ("#
        ));
        let mut sep = qb.separated(", ");
        for c in chunk {
            sep.push_bind(c);
        }
        qb.push(") ORDER BY e.summit_code, e.muni_key, e.pota_code");
        let rows = qb
            .build()
            .fetch_all(pool)
            .await
            .map_err(db_error("select composed pairs"))?;
        for r in rows {
            out.entry(r.get("summit_code"))
                .or_default()
                .push(ComposedPair {
                    muni_key: r.get("muni_key"),
                    pota_code: r.get("pota_code"),
                    park_name: r.get("park_name"),
                    coverage: cov(&r.get::<String, _>("coverage")),
                });
        }
    }
    Ok(out)
}

pub async fn find_summit_map_inputs(
    pool: &SqlitePool,
    codes: &[String],
) -> AppResult<Vec<SummitMapInput>> {
    let mut pairs = find_composed_pairs(pool, codes).await?;
    let mut out = Vec::new();
    for chunk in codes.chunks(IN_CHUNK) {
        let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT summit_code, base_geojson FROM summit_maps \
             WHERE base_geojson IS NOT NULL AND summit_code IN (",
        );
        let mut sep = qb.separated(", ");
        for c in chunk {
            sep.push_bind(c);
        }
        qb.push(") ORDER BY summit_code");
        let rows = qb
            .build()
            .fetch_all(pool)
            .await
            .map_err(db_error("select summit_map inputs"))?;
        for r in rows {
            let code: String = r.get("summit_code");
            out.push(SummitMapInput {
                pairs: pairs.remove(&code).unwrap_or_default(),
                base_geojson: r.get("base_geojson"),
                summit_code: code,
            });
        }
    }
    Ok(out)
}

// ---- 地図 ----

pub async fn upsert_summit_maps(
    pool: &SqlitePool,
    maps: Vec<(SummitMap, String)>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(tx_error("begin upsert_summit_maps"))?;
    for (base, served) in maps {
        sqlx::query(
            r#"
            INSERT INTO summit_maps(summit_code, geojson, base_geojson, base_rev, served_at)
            VALUES($1, $2, $3,
                   (SELECT input_rev FROM summit_estimates_meta WHERE summit_code = $1), $4)
            ON CONFLICT (summit_code) DO UPDATE SET
                geojson = EXCLUDED.geojson, base_geojson = EXCLUDED.base_geojson,
                base_rev = EXCLUDED.base_rev, served_at = EXCLUDED.served_at
        "#,
        )
        .bind(&base.summit_code)
        .bind(served)
        .bind(&base.geojson)
        .bind(now.timestamp())
        .execute(&mut *tx)
        .await
        .map_err(db_error("upsert summit_maps"))?;
    }
    tx.commit()
        .await
        .map_err(tx_error("commit upsert_summit_maps"))?;
    Ok(())
}

pub async fn update_served_maps(
    pool: &SqlitePool,
    maps: Vec<SummitMap>,
    now: DateTime<Utc>,
) -> AppResult<()> {
    let mut tx = pool
        .begin()
        .await
        .map_err(tx_error("begin update_served_maps"))?;
    for m in maps {
        update_served(&mut tx, &m.summit_code, &m.geojson, now).await?;
    }
    tx.commit()
        .await
        .map_err(tx_error("commit update_served_maps"))?;
    Ok(())
}

async fn update_served(
    conn: &mut SqliteConnection,
    code: &str,
    geojson: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query("UPDATE summit_maps SET geojson = $2, served_at = $3 WHERE summit_code = $1")
        .bind(code)
        .bind(geojson)
        .bind(now.timestamp())
        .execute(conn)
        .await
        .map_err(db_error("update served map"))?;
    Ok(())
}

pub async fn find_summit_map(
    pool: &SqlitePool,
    summit_code: &SummitCode,
) -> AppResult<Option<SummitMap>> {
    let row = sqlx::query(
        "SELECT m.summit_code, m.geojson FROM summit_maps AS m \
         JOIN sota_references AS s ON s.summit_code = m.summit_code WHERE m.summit_code = $1",
    )
    .bind(summit_code.inner_ref())
    .fetch_optional(pool)
    .await
    .map_err(db_error("select summit_maps"))?;
    Ok(row.map(|r| SummitMap {
        summit_code: r.get("summit_code"),
        geojson: r.get("geojson"),
    }))
}

pub async fn find_summit_maps(pool: &SqlitePool, prefix: &str) -> AppResult<Vec<SummitMap>> {
    let (lower, upper) = prefix_range(prefix);
    let rows = sqlx::query(
        "SELECT m.summit_code, m.geojson FROM summit_maps AS m \
         JOIN sota_references AS s ON s.summit_code = m.summit_code \
         WHERE m.summit_code >= $1 AND m.summit_code < $2 ORDER BY m.summit_code",
    )
    .bind(lower)
    .bind(upper)
    .fetch_all(pool)
    .await
    .map_err(db_error("select summit_maps by prefix"))?;
    Ok(rows
        .iter()
        .map(|r| SummitMap {
            summit_code: r.get("summit_code"),
            geojson: r.get("geojson"),
        })
        .collect())
}

pub async fn find_map_versions(
    pool: &SqlitePool,
    prefix: &str,
) -> AppResult<Vec<(String, DateTime<Utc>)>> {
    let (lower, upper) = prefix_range(prefix);
    let rows = sqlx::query(
        r#"
        SELECT substr(m.summit_code, 1, instr(m.summit_code, '-') - 1) AS region,
               MAX(m.served_at) AS served_at
        FROM summit_maps AS m
        JOIN sota_references AS s ON s.summit_code = m.summit_code
        WHERE m.summit_code >= $1 AND m.summit_code < $2 AND m.served_at IS NOT NULL
        GROUP BY region ORDER BY region
    "#,
    )
    .bind(lower)
    .bind(upper)
    .fetch_all(pool)
    .await
    .map_err(db_error("select map versions"))?;
    Ok(rows
        .iter()
        .map(|r| (r.get("region"), at(r.get("served_at"))))
        .collect())
}

// ---- 見直し ----

pub async fn find_summit_review_list(
    pool: &SqlitePool,
    prefix: &str,
) -> AppResult<Vec<SummitReviewSummary>> {
    let (lower, upper) = prefix_range(prefix);
    let rows = sqlx::query(
        r#"
        SELECT s.summit_code,
               COALESCE(s.summit_name_j, s.summit_name) AS summit_name,
               s.latitude AS ref_latitude, s.longitude AS ref_longitude,
               m.input_rev AS m_input_rev, m.latitude AS m_latitude, m.longitude AS m_longitude,
               m.park_version AS m_park_version, m.muni_version AS m_muni_version,
               m.munitable_version AS m_munitable_version, m.reasons AS m_reasons,
               m.change_note AS m_change_note, m.computed_at AS m_computed_at,
               EXISTS(SELECT 1 FROM summit_maps AS x WHERE x.summit_code = s.summit_code) AS has_map,
               (SELECT x.base_rev FROM summit_maps AS x WHERE x.summit_code = s.summit_code) AS base_rev,
               COALESCE(a.pairs, 0) AS pairs, COALESCE(a.all_pairs, 0) AS all_pairs,
               COALESCE(a.part_pairs, 0) AS part_pairs, COALESCE(a.none_pairs, 0) AS none_pairs,
               COALESCE(a.flagged_pairs, 0) AS flagged_pairs,
               r.input_rev AS r_input_rev, r.reviewed_by AS r_reviewed_by,
               r.reviewed_at AS r_reviewed_at, r.park_version AS r_park_version,
               r.muni_version AS r_muni_version, r.munitable_version AS r_munitable_version,
               r.note AS r_note
        FROM sota_references AS s
        LEFT JOIN summit_estimates_meta AS m ON m.summit_code = s.summit_code
        LEFT JOIN summit_reviews AS r ON r.summit_code = s.summit_code
        LEFT JOIN (
            SELECT summit_code, COUNT(*) AS pairs,
                   SUM(coverage = 'all') AS all_pairs, SUM(coverage = 'part') AS part_pairs,
                   SUM(coverage = 'none') AS none_pairs,
                   SUM(needs_review <> 0 OR noise <> 0 OR shallow <> 0
                       OR (note IS NOT NULL AND note <> '')) AS flagged_pairs
            FROM summit_pota_estimates GROUP BY summit_code
        ) AS a ON a.summit_code = s.summit_code
        WHERE s.summit_code >= $1 AND s.summit_code < $2
        ORDER BY s.summit_code
    "#,
    )
    .bind(lower)
    .bind(upper)
    .fetch_all(pool)
    .await
    .map_err(db_error("select summit review list"))?;
    Ok(rows
        .iter()
        .map(|r| SummitReviewSummary {
            summit_code: r.get("summit_code"),
            summit_name: r.get("summit_name"),
            ref_latitude: r.get("ref_latitude"),
            ref_longitude: r.get("ref_longitude"),
            meta: meta_from(r, "m_"),
            has_map: r.get::<i64, _>("has_map") != 0,
            base_rev: r.get("base_rev"),
            pairs: r.get("pairs"),
            all_pairs: r.get("all_pairs"),
            part_pairs: r.get("part_pairs"),
            none_pairs: r.get("none_pairs"),
            flagged_pairs: r.get("flagged_pairs"),
            review: review_from(r),
        })
        .collect())
}

pub async fn find_summit_review_detail(
    pool: &SqlitePool,
    summit_code: &SummitCode,
) -> AppResult<Option<SummitReviewDetail>> {
    let code = summit_code.inner_ref();

    let head = sqlx::query(
        r#"
        SELECT s.summit_code, COALESCE(s.summit_name_j, s.summit_name) AS summit_name, s.alt_m,
               s.latitude, s.longitude, s.city_j, s.activation_count
        FROM sota_references AS s WHERE s.summit_code = $1
    "#,
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    .map_err(db_error("select summit head"))?;

    let meta_row = sqlx::query(
        r#"
        SELECT summit_code, input_rev AS m_input_rev, latitude AS m_latitude,
               longitude AS m_longitude, park_version AS m_park_version,
               muni_version AS m_muni_version, munitable_version AS m_munitable_version,
               reasons AS m_reasons, change_note AS m_change_note, computed_at AS m_computed_at
        FROM summit_estimates_meta WHERE summit_code = $1
    "#,
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    .map_err(db_error("select summit meta"))?;

    let sql = format!(
        r#"
        SELECT e.summit_code, e.muni_key, e.muni_name, e.pota_code, e.coverage, e.pct, e.px,
               e.tot_px, e.dist_m, e.noise, e.shallow, e.hint, e.needs_review, e.note,
               {PARK_NAME} AS park_name, (COALESCE({PARK_INACTIVE}, 1) <> 0) AS park_inactive
        FROM summit_pota_estimates AS e
        WHERE e.summit_code = $1 ORDER BY e.muni_key, e.pota_code
    "#
    );
    let pair_rows = sqlx::query(&sql)
        .bind(code)
        .fetch_all(pool)
        .await
        .map_err(db_error("select summit pairs"))?;

    let map_row = sqlx::query(
        "SELECT base_geojson, base_rev, served_at FROM summit_maps WHERE summit_code = $1",
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    .map_err(db_error("select summit map"))?;

    if head.is_none() && pair_rows.is_empty() && map_row.is_none() {
        return Ok(None);
    }

    let verdict_rows = sqlx::query(
        "SELECT muni_key, pota_code, coverage FROM summit_pota_verdicts \
         WHERE summit_code = $1 ORDER BY muni_key, pota_code",
    )
    .bind(code)
    .fetch_all(pool)
    .await
    .map_err(db_error("select verdicts"))?;

    let review_row = sqlx::query(
        r#"
        SELECT summit_code, input_rev AS r_input_rev, reviewed_by AS r_reviewed_by,
               reviewed_at AS r_reviewed_at, park_version AS r_park_version,
               muni_version AS r_muni_version, munitable_version AS r_munitable_version,
               note AS r_note
        FROM summit_reviews WHERE summit_code = $1
    "#,
    )
    .bind(code)
    .fetch_optional(pool)
    .await
    .map_err(db_error("select review"))?;

    let log_rows = sqlx::query(
        "SELECT muni_key, pota_code, before, after, changed_by, changed_at FROM summit_pota_log \
         WHERE summit_code = $1 ORDER BY id DESC LIMIT 200",
    )
    .bind(code)
    .fetch_all(pool)
    .await
    .map_err(db_error("select log"))?;

    let pairs = pair_rows
        .iter()
        .map(|r| SummitPairDetail {
            estimate: SummitPotaEstimate {
                summit_code: r.get("summit_code"),
                muni_key: r.get("muni_key"),
                muni_name: r.get("muni_name"),
                pota_code: r.get("pota_code"),
                coverage: cov(&r.get::<String, _>("coverage")),
                pct: r.get("pct"),
                px: r.get("px"),
                tot_px: r.get("tot_px"),
                dist_m: r.get("dist_m"),
                noise: r.get::<i64, _>("noise") != 0,
                shallow: r.get::<i64, _>("shallow") != 0,
                hint: cov_opt(r.get("hint")),
                needs_review: r.get::<i64, _>("needs_review") != 0,
                note: r.get("note"),
            },
            park_name: r.get("park_name"),
            park_inactive: r.get::<i64, _>("park_inactive") != 0,
        })
        .collect();

    let (summit_name, alt_m, latitude, longitude, city_j, activation_count) = match &head {
        Some(h) => (
            h.get("summit_name"),
            h.get("alt_m"),
            h.get("latitude"),
            h.get("longitude"),
            h.get("city_j"),
            h.get::<i64, _>("activation_count"),
        ),
        None => (code.clone(), 0, None, None, None, 0),
    };

    Ok(Some(SummitReviewDetail {
        summit_code: code.clone(),
        summit_name,
        alt_m,
        latitude,
        longitude,
        city_j,
        activation_count,
        meta: meta_row.as_ref().and_then(|r| meta_from(r, "m_")),
        pairs,
        verdicts: verdict_rows
            .iter()
            .map(|r| SummitPotaVerdict {
                muni_key: r.get("muni_key"),
                pota_code: r.get("pota_code"),
                coverage: cov(&r.get::<String, _>("coverage")),
            })
            .collect(),
        review: review_row.as_ref().and_then(review_from),
        base_geojson: map_row.as_ref().and_then(|r| r.get("base_geojson")),
        base_rev: map_row.as_ref().and_then(|r| r.get("base_rev")),
        served_at: map_row
            .as_ref()
            .and_then(|r| r.get::<Option<i64>, _>("served_at"))
            .map(at),
        log: log_rows
            .iter()
            .map(|r| SummitPotaLogEntry {
                muni_key: r.get("muni_key"),
                pota_code: r.get("pota_code"),
                before: cov_opt(r.get("before")),
                after: cov_opt(r.get("after")),
                changed_by: r.get("changed_by"),
                changed_at: at(r.get("changed_at")),
            })
            .collect(),
    }))
}

async fn push_log(
    conn: &mut SqliteConnection,
    code: &str,
    key: &(String, String),
    before: Option<ParkCoverage>,
    after: Option<ParkCoverage>,
    by: &str,
    now: DateTime<Utc>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO summit_pota_log(summit_code, muni_key, pota_code, before, after, changed_by, changed_at) \
         VALUES($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(code)
    .bind(&key.0)
    .bind(&key.1)
    .bind(before.map(|c| c.as_str()))
    .bind(after.map(|c| c.as_str()))
    .bind(by)
    .bind(now.timestamp())
    .execute(conn)
    .await
    .map_err(db_error("insert summit_pota_log"))?;
    Ok(())
}

async fn read_verdicts(
    conn: &mut SqliteConnection,
    code: &str,
) -> AppResult<HashMap<(String, String), ParkCoverage>> {
    let rows = sqlx::query(
        "SELECT muni_key, pota_code, coverage FROM summit_pota_verdicts WHERE summit_code = $1",
    )
    .bind(code)
    .fetch_all(conn)
    .await
    .map_err(db_error("select prev verdicts"))?;
    Ok(rows
        .iter()
        .map(|r| {
            (
                (r.get("muni_key"), r.get("pota_code")),
                cov(&r.get::<String, _>("coverage")),
            )
        })
        .collect())
}

pub async fn save_summit_review(pool: &SqlitePool, req: SaveSummitReview) -> AppResult<()> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(tx_error("begin save_summit_review"))?;
    let code = req.summit_code.as_str();

    // 1. 推定が取り込み直されていないか
    let meta = sqlx::query(
        "SELECT input_rev, park_version, muni_version, munitable_version \
         FROM summit_estimates_meta WHERE summit_code = $1",
    )
    .bind(code)
    .fetch_optional(&mut *tx)
    .await
    .map_err(db_error("select meta for save"))?;
    let (cur_rev, park, muni, munitable) = match &meta {
        Some(m) => (
            m.get::<String, _>("input_rev"),
            m.get::<String, _>("park_version"),
            m.get::<String, _>("muni_version"),
            m.get::<String, _>("munitable_version"),
        ),
        None => (
            String::new(),
            "unknown".into(),
            "unknown".into(),
            "unknown".into(),
        ),
    };
    if cur_rev != req.expected_input_rev {
        return Err(AppError::Conflict(
            "推定が取り込み直されました。読み直してください".to_string(),
        ));
    }

    // 2. 前と後が違う組を履歴に積む(前にあって後にない組は after = NULL)
    let prev = read_verdicts(&mut tx, code).await?;
    let mut next: HashMap<(String, String), ParkCoverage> = HashMap::new();
    for a in &req.answers {
        next.insert((a.muni_key.clone(), a.pota_code.clone()), a.coverage);
    }
    let mut keys: Vec<&(String, String)> = prev
        .keys()
        .chain(next.keys())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    keys.sort();
    for k in keys {
        let (b, a) = (prev.get(k).copied(), next.get(k).copied());
        if b != a {
            push_log(&mut tx, code, k, b, a, &req.reviewed_by, req.now).await?;
        }
    }

    // 3. 目視を置き換える
    sqlx::query("DELETE FROM summit_pota_verdicts WHERE summit_code = $1")
        .bind(code)
        .execute(&mut *tx)
        .await
        .map_err(db_error("delete verdicts"))?;
    for a in &req.answers {
        sqlx::query(
            "INSERT INTO summit_pota_verdicts(summit_code, muni_key, pota_code, coverage) VALUES($1, $2, $3, $4)",
        )
        .bind(code)
        .bind(&a.muni_key)
        .bind(&a.pota_code)
        .bind(a.coverage.as_str())
        .execute(&mut *tx)
        .await
        .map_err(db_error("insert verdict"))?;
    }

    // 4. 確定の記録(版は今の meta から写す)
    sqlx::query(
        r#"
        INSERT INTO summit_reviews(summit_code, input_rev, reviewed_by, reviewed_at,
            park_version, muni_version, munitable_version, note)
        VALUES($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (summit_code) DO UPDATE SET
            input_rev = EXCLUDED.input_rev, reviewed_by = EXCLUDED.reviewed_by,
            reviewed_at = EXCLUDED.reviewed_at, park_version = EXCLUDED.park_version,
            muni_version = EXCLUDED.muni_version, munitable_version = EXCLUDED.munitable_version,
            note = EXCLUDED.note
    "#,
    )
    .bind(code)
    .bind(&req.expected_input_rev)
    .bind(&req.reviewed_by)
    .bind(req.now.timestamp())
    .bind(park)
    .bind(muni)
    .bind(munitable)
    .bind(&req.note)
    .execute(&mut *tx)
    .await
    .map_err(db_error("upsert review"))?;

    // 5. 配信用の地図
    if let Some(g) = &req.served_geojson {
        update_served(&mut tx, code, g, req.now).await?;
    }

    tx.commit()
        .await
        .map_err(tx_error("commit save_summit_review"))?;
    Ok(())
}

pub async fn delete_summit_review(pool: &SqlitePool, req: DeleteSummitReview) -> AppResult<()> {
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(tx_error("begin delete_summit_review"))?;
    let code = req.summit_code.as_str();

    let prev = read_verdicts(&mut tx, code).await?;
    let mut keys: Vec<&(String, String)> = prev.keys().collect();
    keys.sort();
    for k in keys {
        push_log(
            &mut tx,
            code,
            k,
            prev.get(k).copied(),
            None,
            &req.changed_by,
            req.now,
        )
        .await?;
    }
    sqlx::query("DELETE FROM summit_pota_verdicts WHERE summit_code = $1")
        .bind(code)
        .execute(&mut *tx)
        .await
        .map_err(db_error("delete verdicts"))?;
    sqlx::query("DELETE FROM summit_reviews WHERE summit_code = $1")
        .bind(code)
        .execute(&mut *tx)
        .await
        .map_err(db_error("delete review"))?;
    if let Some(g) = &req.served_geojson {
        update_served(&mut tx, code, g, req.now).await?;
    }

    tx.commit()
        .await
        .map_err(tx_error("commit delete_summit_review"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::migrate::Migrator;
    use std::path::Path;
    use tempfile::tempdir;

    async fn setup() -> (SqlitePool, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        std::fs::File::create(&path).unwrap();
        let pool = SqlitePool::connect(&format!("sqlite:{}", path.display()))
            .await
            .unwrap();
        Migrator::new(Path::new("migrations/sqlite"))
            .await
            .unwrap()
            .run(&pool)
            .await
            .unwrap();
        (pool, dir)
    }

    async fn add_summit(pool: &SqlitePool, code: &str) {
        sqlx::query(
            "INSERT INTO sota_references(summit_code, association_name, region_name, summit_name, \
             alt_m, alt_ft, grid_ref1, grid_ref2, longitude, latitude, points, bonus_points, \
             valid_from, valid_to, activation_count) \
             VALUES($1, 'Japan', 'Kanagawa', 'テスト', 1000, 3280, '', '', 139.0, 35.0, 4, 0, '', '', 3)",
        )
        .bind(code)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn add_park(pool: &SqlitePool, code: &str, inactive: i64) {
        sqlx::query(
            "INSERT INTO pota_references(pota_code, wwff_code, park_name, park_name_j, park_location, \
             park_locid, park_type, park_inactive, \"update\") \
             VALUES($1, '', 'p', '公園', '', '', '', $2, '2026-01-01 00:00:00')",
        )
        .bind(code)
        .bind(inactive)
        .execute(pool)
        .await
        .unwrap();
    }

    fn est(code: &str, key: &str, park: &str, c: ParkCoverage) -> SummitPotaEstimate {
        SummitPotaEstimate {
            summit_code: code.into(),
            muni_key: key.into(),
            muni_name: "市".into(),
            pota_code: park.into(),
            coverage: c,
            pct: None,
            px: None,
            tot_px: None,
            dist_m: None,
            noise: false,
            shallow: false,
            hint: None,
            needs_review: false,
            note: None,
        }
    }

    fn meta(code: &str, rev: &str) -> SummitEstimateMeta {
        SummitEstimateMeta {
            summit_code: code.into(),
            input_rev: rev.into(),
            latitude: Some(35.0),
            longitude: Some(139.0),
            versions: DataVersions {
                park: "p1".into(),
                muni: "m1".into(),
                munitable: "t1".into(),
            },
            reasons: vec![],
            change_note: None,
            computed_at: at(1_700_000_000),
        }
    }

    fn verdict(key: &str, park: &str, c: ParkCoverage) -> SummitPotaVerdict {
        SummitPotaVerdict {
            muni_key: key.into(),
            pota_code: park.into(),
            coverage: c,
        }
    }

    fn save_req(code: &str, rev: &str, answers: Vec<SummitPotaVerdict>) -> SaveSummitReview {
        SaveSummitReview {
            summit_code: code.into(),
            expected_input_rev: rev.into(),
            answers,
            reviewed_by: "a@example.com".into(),
            note: None,
            now: at(1_700_000_100),
            served_geojson: None,
        }
    }

    async fn seed(pool: &SqlitePool) {
        add_summit(pool, "JA/KN-017").await;
        add_park(pool, "JP-1", 0).await;
        add_park(pool, "JP-2", 0).await;
        import_summit_estimates(
            pool,
            vec![
                est("JA/KN-017", "JCC#1", "JP-1", ParkCoverage::Part),
                est("JA/KN-017", "JCC#1", "JP-2", ParkCoverage::None),
            ],
            vec![meta("JA/KN-017", "rev1")],
            false,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn serving_query_looks_parks_up_by_index_not_by_scanning() {
        // pota_references は約 9.7 万行。全行を走査したり一時的な索引を作ったりすると、本番の小さなマシンで /pota が数秒かかる
        let (pool, _d) = setup().await;
        let plan = sqlx::query(&format!("EXPLAIN QUERY PLAN {}", served_sql()))
            .bind("JA/KN-017")
            .fetch_all(&pool)
            .await
            .unwrap();
        let detail: Vec<String> = plan.iter().map(|r| r.get::<String, _>("detail")).collect();
        for d in &detail {
            assert!(!d.contains("SCAN pota_references"), "{detail:?}");
            assert!(!d.contains("AUTOMATIC"), "{detail:?}");
        }
        assert!(
            detail
                .iter()
                .any(|d| d.contains("SEARCH p USING") && d.contains("idx_pota_references_code")),
            "{detail:?}"
        );
    }

    #[tokio::test]
    async fn duplicated_park_rows_are_served_once_and_active_wins() {
        // 同じ pota_code に複数の行(wwff_code 違い)があっても、1 組は 1 行。1 行でも有効なら有効
        let (pool, _d) = setup().await;
        add_summit(&pool, "JA/KN-017").await;
        add_park(&pool, "JP-1", 1).await;
        sqlx::query(
            "INSERT INTO pota_references(pota_code, wwff_code, park_name, park_name_j, park_location, \
             park_locid, park_type, park_inactive, \"update\") \
             VALUES('JP-1', 'JAFF-1', 'p', 'あ公園', '', '', '', 0, '2026-01-01 00:00:00')",
        )
        .execute(&pool)
        .await
        .unwrap();
        import_summit_estimates(
            &pool,
            vec![est("JA/KN-017", "JCC#1", "JP-1", ParkCoverage::All)],
            vec![meta("JA/KN-017", "r")],
            false,
        )
        .await
        .unwrap();
        let rows = find_summit_pota(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].park_name.as_deref(), Some("あ公園")); // MIN
        let d = find_summit_review_detail(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(!d.pairs[0].park_inactive);
        let composed = find_composed_pairs(&pool, &["JA/KN-017".to_string()])
            .await
            .unwrap();
        assert_eq!(composed["JA/KN-017"].len(), 1);
    }

    #[tokio::test]
    async fn serves_estimate_and_excludes_none() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        let rows = find_summit_pota(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pota_code, "JP-1");
        assert!(!rows[0].verified);
    }

    #[tokio::test]
    async fn verdict_overrides_estimate() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        save_summit_review(
            &pool,
            save_req(
                "JA/KN-017",
                "rev1",
                vec![
                    verdict("JCC#1", "JP-1", ParkCoverage::None),
                    verdict("JCC#1", "JP-2", ParkCoverage::All),
                ],
            ),
        )
        .await
        .unwrap();
        let rows = find_summit_pota(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pota_code, "JP-2");
        assert_eq!(rows[0].coverage, ParkCoverage::All);
        assert!(rows[0].verified);
    }

    #[tokio::test]
    async fn inactive_or_unknown_parks_and_unknown_summits_are_not_served() {
        let (pool, _d) = setup().await;
        add_summit(&pool, "JA/KN-017").await;
        add_park(&pool, "JP-OFF", 1).await;
        import_summit_estimates(
            &pool,
            vec![
                est("JA/KN-017", "JCC#1", "JP-OFF", ParkCoverage::All),
                est("JA/KN-017", "JCC#1", "JP-NONE", ParkCoverage::All),
                est("JA/KN-999", "JCC#1", "JP-OFF", ParkCoverage::All),
            ],
            vec![meta("JA/KN-017", "r"), meta("JA/KN-999", "r")],
            false,
        )
        .await
        .unwrap();
        for code in ["JA/KN-017", "JA/KN-999"] {
            assert!(find_summit_pota(&pool, &SummitCode::new(code.into()))
                .await
                .unwrap()
                .is_empty());
        }
    }

    #[tokio::test]
    async fn import_keeps_verdicts_and_counts_rereview() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        save_summit_review(
            &pool,
            save_req(
                "JA/KN-017",
                "rev1",
                vec![
                    verdict("JCC#1", "JP-1", ParkCoverage::All),
                    verdict("JCC#1", "JP-2", ParkCoverage::None),
                ],
            ),
        )
        .await
        .unwrap();

        // 同じ input_rev の取り込みでは、要再確認にならない
        let n = import_summit_estimates(
            &pool,
            vec![est("JA/KN-017", "JCC#1", "JP-1", ParkCoverage::Part)],
            vec![meta("JA/KN-017", "rev1")],
            false,
        )
        .await
        .unwrap();
        assert_eq!(n, 0);

        // 候補が減り input_rev が変わっても、目視は残り、要再確認の数が返る
        let n = import_summit_estimates(
            &pool,
            vec![est("JA/KN-017", "JCC#1", "JP-1", ParkCoverage::Part)],
            vec![meta("JA/KN-017", "rev2")],
            false,
        )
        .await
        .unwrap();
        assert_eq!(n, 1);
        let d = find_summit_review_detail(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(d.verdicts.len(), 2);
        assert_eq!(d.pairs.len(), 1);
        assert_eq!(d.review.unwrap().input_rev, "rev1");
    }

    #[tokio::test]
    async fn partial_import_leaves_other_summits_and_full_import_keeps_verdicts() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        add_summit(&pool, "JA/KN-018").await;
        import_summit_estimates(
            &pool,
            vec![est("JA/KN-018", "JCC#1", "JP-1", ParkCoverage::All)],
            vec![meta("JA/KN-018", "r")],
            false,
        )
        .await
        .unwrap();
        save_summit_review(
            &pool,
            save_req(
                "JA/KN-017",
                "rev1",
                vec![
                    verdict("JCC#1", "JP-1", ParkCoverage::All),
                    verdict("JCC#1", "JP-2", ParkCoverage::None),
                ],
            ),
        )
        .await
        .unwrap();

        // 山頂を絞った取り込み: JA/KN-017 は変わらない
        import_summit_estimates(
            &pool,
            vec![est("JA/KN-018", "JCC#1", "JP-1", ParkCoverage::Part)],
            vec![meta("JA/KN-018", "r2")],
            false,
        )
        .await
        .unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM summit_pota_estimates WHERE summit_code = 'JA/KN-017'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 2);

        sqlx::query("INSERT INTO summit_maps(summit_code, geojson) VALUES('JA/KN-017', '{}')")
            .execute(&pool)
            .await
            .unwrap();

        // full: ファイルにない JA/KN-017 の推定・meta は消えるが、目視・確定・地図は消えない
        import_summit_estimates(
            &pool,
            vec![est("JA/KN-018", "JCC#1", "JP-1", ParkCoverage::Part)],
            vec![meta("JA/KN-018", "r2")],
            true,
        )
        .await
        .unwrap();
        for (table, want) in [
            ("summit_pota_estimates", 0),
            ("summit_estimates_meta", 0),
            ("summit_pota_verdicts", 2),
            ("summit_reviews", 1),
            ("summit_maps", 1),
        ] {
            let n: i64 = sqlx::query_scalar(&format!(
                "SELECT COUNT(*) FROM {table} WHERE summit_code = 'JA/KN-017'"
            ))
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(n, want, "{table}");
        }
    }

    #[tokio::test]
    async fn save_conflicts_when_input_rev_changed_and_logs_changes() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        let err = save_summit_review(&pool, save_req("JA/KN-017", "old", vec![]))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));

        let a = vec![
            verdict("JCC#1", "JP-1", ParkCoverage::All),
            verdict("JCC#1", "JP-2", ParkCoverage::None),
        ];
        save_summit_review(&pool, save_req("JA/KN-017", "rev1", a.clone()))
            .await
            .unwrap();
        // 同じ答えの保存は履歴を増やさない
        save_summit_review(&pool, save_req("JA/KN-017", "rev1", a))
            .await
            .unwrap();
        // 1 組だけ変える
        save_summit_review(
            &pool,
            save_req(
                "JA/KN-017",
                "rev1",
                vec![
                    verdict("JCC#1", "JP-1", ParkCoverage::Part),
                    verdict("JCC#1", "JP-2", ParkCoverage::None),
                ],
            ),
        )
        .await
        .unwrap();
        let d = find_summit_review_detail(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(d.log.len(), 3);
        assert_eq!(d.log[0].before, Some(ParkCoverage::All));
        assert_eq!(d.log[0].after, Some(ParkCoverage::Part));
        assert_eq!(d.review.unwrap().versions.park, "p1");
    }

    #[tokio::test]
    async fn delete_review_logs_and_clears() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        save_summit_review(
            &pool,
            save_req(
                "JA/KN-017",
                "rev1",
                vec![
                    verdict("JCC#1", "JP-1", ParkCoverage::All),
                    verdict("JCC#1", "JP-2", ParkCoverage::None),
                ],
            ),
        )
        .await
        .unwrap();
        delete_summit_review(
            &pool,
            DeleteSummitReview {
                summit_code: "JA/KN-017".into(),
                changed_by: "a@example.com".into(),
                now: at(1_700_000_200),
                served_geojson: None,
            },
        )
        .await
        .unwrap();
        let d = find_summit_review_detail(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap()
            .unwrap();
        assert!(d.verdicts.is_empty());
        assert!(d.review.is_none());
        assert_eq!(d.log.len(), 4);
        assert_eq!(d.log[0].after, None);
        let rows = find_summit_pota(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap();
        assert!(!rows[0].verified);
    }

    #[tokio::test]
    async fn maps_upsert_versions_and_serving() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        add_summit(&pool, "JA/TK-001").await;
        let base = SummitMap {
            summit_code: "JA/KN-017".into(),
            geojson: "{\"base\":1}".into(),
        };
        upsert_summit_maps(
            &pool,
            vec![(base, "{\"served\":1}".into())],
            at(1_700_000_000),
        )
        .await
        .unwrap();
        // sota_references にない山頂の地図は返さない
        sqlx::query(
            "INSERT INTO summit_maps(summit_code, geojson, served_at) VALUES('JA/ZZ-001', '{}', 5)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let maps = find_summit_maps(&pool, "JA/").await.unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].geojson, "{\"served\":1}");

        let inputs = find_summit_map_inputs(&pool, &["JA/KN-017".to_string()])
            .await
            .unwrap();
        assert_eq!(inputs[0].base_geojson, "{\"base\":1}");
        assert_eq!(inputs[0].pairs.len(), 2);

        update_served_maps(
            &pool,
            vec![SummitMap {
                summit_code: "JA/KN-017".into(),
                geojson: "{\"served\":2}".into(),
            }],
            at(1_700_000_500),
        )
        .await
        .unwrap();
        let v = find_map_versions(&pool, "JA/").await.unwrap();
        assert_eq!(v, vec![("JA/KN".to_string(), at(1_700_000_500))]);
        let d = find_summit_review_detail(&pool, &SummitCode::new("JA/KN-017".into()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(d.base_rev.as_deref(), Some("rev1"));
    }

    #[tokio::test]
    async fn review_list_counts_pairs() {
        let (pool, _d) = setup().await;
        seed(&pool).await;
        let list = find_summit_review_list(&pool, "JA/KN-").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].pairs, 2);
        assert_eq!(list[0].part_pairs, 1);
        assert_eq!(list[0].none_pairs, 1);
        assert!(!list[0].has_map);
        assert!(list[0].review.is_none());
        assert_eq!(list[0].meta.as_ref().unwrap().input_rev, "rev1");
    }
}
