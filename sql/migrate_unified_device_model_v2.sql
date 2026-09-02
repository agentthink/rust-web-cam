-- Migration: Unified Device Model V2
-- Removes stream_key from devices, adds channel_extended and is_default
-- stream_key now belongs to Stream table, not Device

BEGIN;

-- Remove stream_key column (stream_key now belongs to Stream table)
ALTER TABLE devices DROP COLUMN IF EXISTS stream_key;

-- Add new columns for channel attributes
ALTER TABLE devices ADD COLUMN IF NOT EXISTS channel_extended JSONB DEFAULT '{}';
ALTER TABLE devices ADD COLUMN IF NOT EXISTS is_default BOOLEAN DEFAULT FALSE;

-- Add indexes for new columns
CREATE INDEX IF NOT EXISTS idx_devices_is_default ON devices(is_default);
CREATE INDEX IF NOT EXISTS idx_devices_channel_extended ON devices USING GIN (channel_extended);

-- Set is_default = true for the first channel of each parent device (if any channels exist)
-- This is an optional migration step to mark a default channel
WITH ranked_channels AS (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY parent_device_tag ORDER BY created_at) as rn
    FROM devices
    WHERE is_channel = true
)
UPDATE devices d SET is_default = true
FROM ranked_channels rc
WHERE d.id = rc.id AND rc.rn = 1;

COMMIT;
