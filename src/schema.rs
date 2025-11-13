pub const SCHEMA: &'static str = r#"

CREATE TABLE IF NOT EXISTS station (
    id SERIAL PRIMARY KEY,
    address TEXT NOT NULL,
    coords POINT NOT NULL
);

CREATE TABLE IF NOT EXISTS trip (
    id UUID PRIMARY KEY,
    source INTEGER NOT NULL REFERENCES station (id),
    destination INTEGER NOT NULL REFERENCES station (id)
);

CREATE TABLE IF NOT EXISTS request (
    id UUID PRIMARY KEY,
    trip_id UUID REFERENCES trip (id),
    source INTEGER REFERENCES station (id) NOT NULL,
    destination INTEGER REFERENCES station (id) NOT NULL
);

CREATE TABLE IF NOT EXISTS segment (
    s1 INTEGER REFERENCES station (id) NOT NULL,
    s2 INTEGER REFERENCES station (id) NOT NULL,
    points POINT[] NOT NULL,
    distance INTEGER NOT NULL,
    time INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS path (
    trip_id UUID REFERENCES trip (id) NOT NULL,
    station_id INTEGER REFERENCES station (id) NOT NULL,
    index INTEGER NOT NULL
);

"#;
