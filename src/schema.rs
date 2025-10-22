pub const SCHEMA: &'static str = r#"

CREATE TABLE IF NOT EXISTS route (
    id SERIAL PRIMARY KEY,
    waypoints POINT[] NOT NULL
);

CREATE TABLE IF NOT EXISTS station (
    id INTEGER PRIMARY KEY,
    coord POINT NOT NULL,
    address TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS graph (
    route_id INTEGER NOT NULL REFERENCES route(id),
    station_id INTEGER NOT NULL REFERENCES station(id),
    ord INTEGER,
    PRIMARY KEY (route_id, station_id)
);

CREATE TABLE IF NOT EXISTS template (
    id SERIAL PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS template (
    template_id INTEGER NOT NULL REFERENCES template(id),
    route_id INTEGER NOT NULL REFERENCES route(id),
    PRIMARY KEY (template_id, route_id)
);

"#;
