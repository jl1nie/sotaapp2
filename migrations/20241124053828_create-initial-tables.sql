-- Add migration script here
CREATE TABLE PotaCSVData (
    id INTEGER PRIMARY KEY,
    potaref TEXT NOT NULL,
    name TEXT,
    location TEXT,
    locid TEXT,
    parktype TEXT,
    namek TEXT,
    lat REAL NOT NULL,
    lng REAL NOT NULL,
    updates TEXT
);
