-- Add up migration script here
CREATE EXTENSION IF NOT EXISTS postgis;

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
    coordinates GEOMETRY(Point, 4326),
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
CREATE INDEX IF NOT EXISTS idx_sota_references_coordinate ON sota_references USING GIST (coordinates);

CREATE TABLE IF NOT EXISTS pota_references (
    pota_code VARCHAR(255) NOT NULL PRIMARY KEY,
    park_name VARCHAR(255) NOT NULL,
    park_name_j VARCHAR(255) NOT NULL,
    park_location VARCHAR(255) NOT NULL,
    park_locid VARCHAR(255) NOT NULL,
    park_type VARCHAR(255) NOT NULL,
    park_status BOOLEAN NOT NULL,
    park_area INTEGER,
    coordinates GEOMETRY(Point, 4326)
);

CREATE INDEX IF NOT EXISTS idx_pota_references_code ON pota_references (pota_code,park_name,park_name_j);
CREATE INDEX IF NOT EXISTS idx_pota_references_coordinate ON pota_references USING GIST (coordinates);

CREATE TABLE IF NOT EXISTS wwff_references (
    wwff_code VARCHAR(255) NOT NULL PRIMARY KEY,
    pota_code VARCHAR(255)
);

CREATE INDEX IF NOT EXISTS idx_wwff_references_code ON wwff_references (pota_code,wwff_code);

CREATE TABLE IF NOT EXISTS pota_activator_log (
    user_id UUID,
    dx_entity VARCHAR(255),
    "location" VARCHAR(255),
    hasc VARCHAR(255),
    pota_code VARCHAR(255) NOT NULL PRIMARY KEY,
    park_name VARCHAR(255),
    first_qso_date TIMESTAMPTZ,
    attempts INTEGER,
    activations INTEGER,
    qsos INTEGER,
    upload TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_pota_activator_log ON pota_activator_log (user_id,pota_code);
CREATE INDEX IF NOT EXISTS idx_pota_activator_log_date ON pota_activator_log (upload);

CREATE TABLE IF NOT EXISTS pota_hunter_log (
    user_id UUID,
    dx_entity VARCHAR(255),
    "location" VARCHAR(255),
    hasc VARCHAR(255),
    pota_code VARCHAR(255) NOT NULL PRIMARY KEY,
    park_name VARCHAR(255),
    first_qso_date TIMESTAMPTZ,
    qsos INTEGER,
    upload TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_pota_hunter_log ON pota_hunter_log (user_id,pota_code);
CREATE INDEX IF NOT EXISTS idx_pota_hunter_log_date ON pota_hunter_log (upload);

CREATE TABLE IF NOT EXISTS alerts (
    program INTEGER NOT NULL,
    alert_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    reference VARCHAR(255) NOT NULL,
    reference_detail VARCHAR(255) NOT NULL,
    "location" VARCHAR(255) NOT NULL,
    activator VARCHAR(255) NOT NULL,
    activator_name VARCHAR(255),
    start_time TIMESTAMPTZ NOT NULL,
    end_time   TIMESTAMPTZ,
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
    spot_time TIMESTAMPTZ NOT nULL,
    frequency VARCHAR(255) NOT NULL,
    mode VARCHAR(255) NOT NULL,
    spotter VARCHAR(255) NOT NULL,
    comment VARCHAR(255),
    PRIMARY KEY(program, spot_id)
);

CREATE INDEX IF NOT EXISTS idx_spots_id ON spots (program, spot_id);
CREATE INDEX IF NOT EXISTS idx_spots_time ON spots (spot_time DESC);
