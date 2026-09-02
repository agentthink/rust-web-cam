-- Add channel association columns to streams, sessions, recordings tables
-- These columns link entities to channels (device_tag + channel_tag) instead of just device_id

-- ============================================
-- Streams table: add device_tag and channel_tag
-- ============================================
ALTER TABLE streams ADD COLUMN IF NOT EXISTS device_tag VARCHAR(50);
ALTER TABLE streams ADD COLUMN IF NOT EXISTS channel_tag VARCHAR(50);

-- Create index for channel lookups
CREATE INDEX IF NOT EXISTS idx_streams_device_tag ON streams(device_tag);
CREATE INDEX IF NOT EXISTS idx_streams_channel_tag ON channels(channel_tag);

-- ============================================
-- Sessions table: add device_tag and channel_tag
-- ============================================
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS device_tag VARCHAR(50);
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS channel_tag VARCHAR(50);

-- Create index for channel lookups
CREATE INDEX IF NOT EXISTS idx_sessions_device_tag ON sessions(device_tag);
CREATE INDEX IF NOT EXISTS idx_sessions_channel_tag ON sessions(channel_tag);

-- ============================================
-- Recordings table: add device_tag and channel_tag
-- ============================================
ALTER TABLE recordings ADD COLUMN IF NOT EXISTS device_tag VARCHAR(50);
ALTER TABLE recordings ADD COLUMN IF NOT EXISTS channel_tag VARCHAR(50);

-- Create index for channel lookups
CREATE INDEX IF NOT EXISTS idx_recordings_device_tag ON recordings(device_tag);
CREATE INDEX IF NOT EXISTS idx_recordings_channel_tag ON recordings(channel_tag);
