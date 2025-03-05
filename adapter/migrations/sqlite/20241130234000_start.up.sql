-- Add up migration script here
CREATE TABLE IF NOT EXISTS sota_references (
    summit_code VARCHAR(255) NOT NULL PRIMARY KEY,
    association_name VARCHAR(255) NOT NULL,
    region_name VARCHAR(255) NOT NULL,
    summit_name VARCHAR(255) NOT NULL,
    summit_name_j VARCHAR(255),
    city VARCHAR(255),
    city_j VARCHAR(255),
    alt_m INTEGER NOT NULL,
    alt_ft INTEGER NOT NULL,
    grid_ref1 VARCHAR(255) NOT NULL,
    grid_ref2 VARCHAR(255) NOT NULL,
    longitude REAL,
    latitude REAL,
    maidenhead VARCHAR(16),
    points INTEGER NOT NULL,
    bonus_points INTEGER NOT NULL,
    valid_from VARCHAR(255) NOT NULL,
    valid_to VARCHAR(255) NOT NULL,
    activation_count INTEGER NOT NULL,
    activation_date VARCHAR(255),
    activation_call VARCHAR(255)
);

CREATE INDEX IF NOT EXISTS idx_sota_references_summit_code ON sota_references (summit_code,summit_name,summit_name_j);
CREATE INDEX IF NOT EXISTS idex_sota_reference_alt ON sota_references (alt_m DESC);
CREATE INDEX IF NOT EXISTS idx_sota_references_coordinate ON sota_references (longitude, latitude);

CREATE TABLE IF NOT EXISTS sota_log (
    user_id VARCHAR(255),
    my_callsign VARCHAR(255) NOT NULL,
    operator VARCHAR(255) NOT NULL,
    my_summit_code VARCHAR(255),
    time DATETIME NOT NULL,
    frequency VARCHAR(255) NOT NULL,
    mode VARCHAR(255) NOT NULL,
    his_callsign VARCHAR(255) NOT NULL,
    his_summit_code VARCHAR(255),
    comment VARCHAR(255),
    "update" DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_operator_sota_log ON sota_log (operator, user_id);
CREATE INDEX IF NOT EXISTS idx_time_sota_log ON sota_log(time);

CREATE TABLE IF NOT EXISTS pota_references (
    pota_code VARCHAR(255),
    wwff_code VARCHAR(255),
    park_name VARCHAR(255) NOT NULL,
    park_name_j VARCHAR(255) NOT NULL,
    park_location VARCHAR(255) NOT NULL,
    park_locid VARCHAR(255) NOT NULL,
    park_type VARCHAR(255) NOT NULL,
    park_inactive INTEGER NOT NULL,
    park_area INTEGER,
    longitude REAL,
    latitude REAL,
    maidenhead VARCHAR(16),
    "update" DATETIME NOT NULL,
    PRIMARY KEY(pota_code, wwff_code)
);

CREATE INDEX IF NOT EXISTS idx_pota_references_code ON pota_references (pota_code,wwff_code, park_name,park_name_j);
CREATE INDEX IF NOT EXISTS idx_pota_references_coordinate ON pota_references (longitude, latitude);

CREATE TABLE IF NOT EXISTS pota_log (
    log_id UUID NOT NULL,
    dx_entity VARCHAR(255) NOT NULL,
    "location" VARCHAR(255) NOT NULL,
    hasc VARCHAR(255) NOT NULL,
    pota_code VARCHAR(255) NOT NULL,
    park_name VARCHAR(255) NOT NULL,
    first_qso_date TEXT NOT NULL,
    attempts INTEGER,
    activations INTEGER,
    qsos INTEGER,
    PRIMARY KEY(log_id, pota_code)
);

CREATE INDEX IF NOT EXISTS idx_pota_log ON pota_log (log_id,pota_code);

CREATE TABLE IF NOT EXISTS pota_log_user (
    user_id  UUID,
    log_id UUID NOT NULL,
    log_kind VARCHAR(255),
    "update" DATETIME NOT NULL,
    PRIMARY KEY(log_id)
);

CREATE INDEX IF NOT EXISTS idx_pota_log_user ON pota_log_user (user_id, log_id, "update");

CREATE TABLE IF NOT EXISTS alerts (
    program INTEGER NOT NULL,
    alert_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    reference VARCHAR(255) NOT NULL,
    reference_detail VARCHAR(255) NOT NULL,
    "location" VARCHAR(255) NOT NULL,
    activator VARCHAR(255) NOT NULL,
    activator_name VARCHAR(255),
    operator VARCHAR(255),
    start_time DATETIME NOT NULL,
    end_time   DATETIME,
    frequencies  VARCHAR(255) NOT NULL,
    comment  VARCHAR(255),
    poster VARCHAR(255),
    PRIMARY KEY(program, alert_id)
);

CREATE INDEX IF NOT EXISTS idx_alerts_id ON alerts (program, alert_id);
CREATE INDEX IF NOT EXISTS idx_alerts_time ON alerts (start_time DESC);

CREATE TABLE IF NOT EXISTS spots (
    program INTEGER NOT NULL,
    spot_id INTEGER NOT NULL,
    reference VARCHAR(255) NOT NULL,
    reference_detail VARCHAR(255) NOT NULL,
    activator VARCHAR(255) NOT NULL,
    activator_name VARCHAR(255),
    operator VARCHAR(255),
    spot_time DATETIME NOT nULL,
    frequency VARCHAR(255) NOT NULL,
    mode VARCHAR(255) NOT NULL,
    spotter VARCHAR(255) NOT NULL,
    comment VARCHAR(255),
    PRIMARY KEY(program, spot_id)
);

CREATE INDEX IF NOT EXISTS idx_spots_id ON spots (program, spot_id);
CREATE INDEX IF NOT EXISTS idx_spots_time ON spots (spot_time DESC);

CREATE TABLE IF NOT EXISTS municipality_century_codes(
    muni_code INTEGER NOT NULL PRIMARY KEY,
    prefecture VARCHAR(255) NOT NULL,
    municipality VARCHAR(255) NOT NULL,
    jcc_code VARCHAR(255),
    ward_code VARCHAR(255),
    jcc_text VARCHAR(255),
    jcg_code VARCHAR(255),
    jcg_text VARCHAR(255),
    hamlog_code VARCHAR(255)
);

CREATE INDEX IF NOT EXISTS idx_municipality_century_codes_muni_code ON municipality_century_codes (muni_code);

CREATE TABLE IF NOT EXISTS aprs_log (
    time DATETIME NOT NULL,
    callsign VARCHAR(255) NOT NULL,
    ssid INTEGER NOT NULL,
    destination VARCHAR(255) NOT NULL,
    distance REAL NOT NULL,
    message VARCHAR(255),
    state INTEGER NOT NULL,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_aprs_log_timestamp ON aprs_log (time DESC);
CREATE INDEX IF NOT EXISTS idx_aprs_log_callsign ON aprs_log (callsign);
