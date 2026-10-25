-- 山頂の簡易地図と POTA の判定の管理画面(docs/summit-map-admin-design.md 3 章)

-- 管理画面に入れる人と役割(admin / reviewer)。メールは小文字。行はここでは入れない
CREATE TABLE IF NOT EXISTS admin_users (
    email VARCHAR(255) NOT NULL PRIMARY KEY,
    role VARCHAR(16) NOT NULL,
    created_at INTEGER NOT NULL
);

-- 推定: 候補の組すべて(none も含む)と、答え・根拠の数値(バッチが入れる)
CREATE TABLE IF NOT EXISTS summit_pota_estimates (
    summit_code VARCHAR(255) NOT NULL,
    muni_key VARCHAR(255) NOT NULL,
    muni_name VARCHAR(255) NOT NULL,
    pota_code VARCHAR(255) NOT NULL,
    -- all / part / none
    coverage VARCHAR(16) NOT NULL,
    pct REAL,
    px INTEGER,
    tot_px INTEGER,
    dist_m REAL,
    noise INTEGER NOT NULL DEFAULT 0,
    shallow INTEGER NOT NULL DEFAULT 0,
    hint VARCHAR(16),
    needs_review INTEGER NOT NULL DEFAULT 0,
    note TEXT,
    PRIMARY KEY (summit_code, muni_key, pota_code)
);

-- 推定を計算したときの入力。山頂ごとに 1 行(バッチが入れる)
CREATE TABLE IF NOT EXISTS summit_estimates_meta (
    summit_code VARCHAR(255) NOT NULL PRIMARY KEY,
    input_rev VARCHAR(64) NOT NULL,
    latitude REAL,
    longitude REAL,
    park_version VARCHAR(64) NOT NULL,
    muni_version VARCHAR(64) NOT NULL,
    munitable_version VARCHAR(64) NOT NULL,
    -- muni,near,shallow の並び
    reasons TEXT,
    change_note TEXT,
    computed_at INTEGER NOT NULL
);

-- 目視の答え(管理画面。推定の取り込みでは触らない)
CREATE TABLE IF NOT EXISTS summit_pota_verdicts (
    summit_code VARCHAR(255) NOT NULL,
    muni_key VARCHAR(255) NOT NULL,
    pota_code VARCHAR(255) NOT NULL,
    coverage VARCHAR(16) NOT NULL,
    PRIMARY KEY (summit_code, muni_key, pota_code)
);

-- 山頂ごとの確定の記録
CREATE TABLE IF NOT EXISTS summit_reviews (
    summit_code VARCHAR(255) NOT NULL PRIMARY KEY,
    input_rev VARCHAR(64) NOT NULL,
    reviewed_by VARCHAR(255) NOT NULL,
    reviewed_at INTEGER NOT NULL,
    park_version VARCHAR(64) NOT NULL,
    muni_version VARCHAR(64) NOT NULL,
    munitable_version VARCHAR(64) NOT NULL,
    note TEXT
);

-- 答えの変更の履歴
CREATE TABLE IF NOT EXISTS summit_pota_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    summit_code VARCHAR(255) NOT NULL,
    muni_key VARCHAR(255) NOT NULL,
    pota_code VARCHAR(255) NOT NULL,
    before VARCHAR(16),
    after VARCHAR(16),
    changed_by VARCHAR(255) NOT NULL,
    changed_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_summit_pota_log_summit ON summit_pota_log(summit_code, id);

-- 今の summit_pota を推定に移す。verified=1 の行も推定として入れる(目視は移行で入れ直す。それまで配信の verified は false)
INSERT INTO summit_pota_estimates (summit_code, muni_key, muni_name, pota_code, coverage)
SELECT summit_code, muni_key, muni_name, pota_code, coverage FROM summit_pota;

-- 山頂ごとの meta。旧データなので版は 'legacy'
INSERT INTO summit_estimates_meta
    (summit_code, input_rev, latitude, longitude, park_version, muni_version, munitable_version, computed_at)
SELECT c.summit_code, 'legacy', s.latitude, s.longitude, 'legacy', 'legacy', 'legacy', strftime('%s','now')
FROM (
    SELECT summit_code FROM summit_pota
    UNION
    SELECT summit_code FROM summit_maps
) AS c
LEFT JOIN sota_references AS s ON s.summit_code = c.summit_code;

DROP TABLE summit_pota;

-- geojson は配信用、base_geojson は下地。base_rev は下地を入れたときの推定の input_rev、served_at は配信用を作り直した日時
ALTER TABLE summit_maps ADD COLUMN base_geojson TEXT;
ALTER TABLE summit_maps ADD COLUMN base_rev VARCHAR(64);
ALTER TABLE summit_maps ADD COLUMN served_at INTEGER;
UPDATE summit_maps SET base_geojson = geojson, base_rev = 'legacy', served_at = strftime('%s','now');
