-- Migration: Remove stream_key from recordings, update to device_tag/channel_tag
-- Date: 2026-09-01
-- Description: Recording domain now uses device_tag and channel_tag, stream_key is computed

-- ============================================================================
-- STEP 1: Add device_tag and channel_tag columns to recordings
-- ============================================================================

ALTER TABLE recordings ADD COLUMN IF NOT EXISTS device_tag VARCHAR(255);
ALTER TABLE recordings ADD COLUMN IF NOT EXISTS channel_tag VARCHAR(255);

-- ============================================================================
-- STEP 2: Copy data from device_id to device_tag (requires join with devices table)
-- ============================================================================

UPDATE recordings SET device_tag = (
    SELECT d.device_tag FROM devices d WHERE d.id = recordings.device_id
) WHERE device_tag IS NULL;

UPDATE recordings SET channel_tag = 'main' WHERE channel_tag IS NULL;

-- ============================================================================
-- STEP 3: Remove old columns
-- ============================================================================

ALTER TABLE recordings DROP COLUMN IF EXISTS device_id;
ALTER TABLE recordings DROP COLUMN IF EXISTS stream_key;

-- ============================================================================
-- STEP 4: Verify changes
-- ============================================================================

-- Check recordings columns
-- SELECT column_name FROM information_schema.columns WHERE table_name = 'recordings' ORDER BY ordinal_position;

-- Expected columns: id, device_tag, channel_tag, media_server_name, state, format,
--                  output_path, file_size, duration_secs, created_at, started_at, stopped_at,
--                  error_message, labels, filename
