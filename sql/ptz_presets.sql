CREATE TABLE IF NOT EXISTS ptz_presets (
    id UUID PRIMARY KEY,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    token VARCHAR(255) NOT NULL,
    position_pan DOUBLE PRECISION,
    position_tilt DOUBLE PRECISION,
    position_zoom DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(device_id, token)
);

CREATE INDEX IF NOT EXISTS idx_ptz_presets_device ON ptz_presets(device_id);
