CREATE TABLE IF NOT EXISTS player_window_layouts (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    rows INTEGER NOT NULL DEFAULT 2,
    cols INTEGER NOT NULL DEFAULT 2,
    layout_json JSONB NOT NULL DEFAULT '[]',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_player_layouts_name ON player_window_layouts(name);
