-- Add up migration script here
CREATE EXTENSION IF NOT EXISTS postgis;
CREATE TABLE IF NOT EXISTS sota_references (
 summit_code VARCHAR(255) NOT NULL UNIQUE,
 association_name VARCHAR(255) NOT NULL,
 region_name VARCHAR(255) NOT NULL,
 summit_name VARCHAR(255) NOT NULL,
 summit_name_j VARCHAR(255),
 city VARCHAR(255),
 city_j VARCHAR(255),
 alt_m INTEGER NOT NULL,
 alt_ft INTEGER NOT NULL,
 grid_ref1 TEXT NOT NULL,
 grid_ref2 TEXT NOT NULL,
 coordinates GEOMETRY(Point, 4326),
 points INTEGER NOT NULL,
 bonus_points INTEGER NOT NULL,
 valid_from DATE NOT NULL,
 valid_to DATE NOT NULL,
 activation_count INTEGER NOT NULL,
 activation_date DATE,
 activation_call VARCHAR(255)
);

CREATE INDEX IF NOT EXISTS idx_sota_references_summit_code ON sota_references (summit_code,summit_name,summit_name_j);
CREATE INDEX IF NOT EXISTS idex_sota_reference_alt ON sota_references (alt_m DESC);
CREATE INDEX IF NOT EXISTS idx_sota_references_coordinate ON sota_references USING GIST (coordinates);
