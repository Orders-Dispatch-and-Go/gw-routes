pub const SCHEMA: &'static str = r#"
CREATE TABLE IF NOT EXISTS routes (
    id SERIAL PRIMARY KEY,
    start DATETIME NOT NULL,
    end DATETIME NOT NULL,
    graph BIGINT[] NOT NULL,
    waypoits REAL[][] NOT NULL
);"#;
