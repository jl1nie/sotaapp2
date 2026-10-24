-- 山頂ごとに、選ぶ市区町村(JCC/JCG)で当たる POTA の公園。空間計算は手元のバッチで行い、ここには結果だけを入れる
CREATE TABLE IF NOT EXISTS summit_pota (
    summit_code VARCHAR(255) NOT NULL,
    muni_key VARCHAR(255) NOT NULL,
    muni_name VARCHAR(255) NOT NULL,
    pota_code VARCHAR(255) NOT NULL,
    -- all: 市区町村の中のゾーンが全部公園に入る / part: 一部だけ入る
    coverage VARCHAR(16) NOT NULL,
    -- 1: 目視で確かめた答え / 0: 推定
    verified INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (summit_code, muni_key, pota_code)
);

-- 山頂ごとの簡易地図(GeoJSON の FeatureCollection)
CREATE TABLE IF NOT EXISTS summit_maps (
    summit_code VARCHAR(255) NOT NULL PRIMARY KEY,
    geojson TEXT NOT NULL
);
