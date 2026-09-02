CREATE TABLE IF NOT EXISTS ptz_control_logs (
    id UUID PRIMARY KEY,
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    command VARCHAR(100) NOT NULL,
    speed INTEGER NOT NULL DEFAULT 50,
    result VARCHAR(50) NOT NULL DEFAULT 'success',
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ptz_logs_device ON ptz_control_logs(device_id);
CREATE INDEX IF NOT EXISTS idx_ptz_logs_user ON ptz_control_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_ptz_logs_created ON ptz_control_logs(created_at DESC);
