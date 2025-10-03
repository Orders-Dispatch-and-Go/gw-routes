pub const SCHEMA: &'static str = r#"

CREATE TABLE IF NOT EXISTS route (
    id SERIAL PRIMARY KEY,
    creation_time TIMESTAMP NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    graph BIGINT[] NOT NULL,
    waypoints REAL[][] NOT NULL,
    is_cancelled BOOLEAN NOT NULL DEFAULT false,
    max_weight INTEGER NOT NULL,
    free_space INTEGER,
  	description TEXT,
    vehicle_id INTEGER,
    driver_id INTEGER,
    allowed_cargo TEXT,
    min_price NUMERIC(10, 2)
);

"#;
