-- Migration: Remove device_id from streams and sessions, rename columns
-- Date: 2026-08-31
-- Description: Stream and Session domains now use device_tag/channel_tag for identification

-- ============================================================================
-- STEP 1: Update streams table
-- ============================================================================

-- Add device_tag and channel_tag columns if they don't exist (from previous migration)
-- This migration removes device_id column

ALTER TABLE streams DROP COLUMN IF EXISTS device_id;

-- Verify the streams table structure
-- Expected columns: id, device_tag, channel_tag, media_server_tag, stream_key, app, token, state, 
--                  retry_count, max_retries, last_error, viewer_count, bandwidth_in, bandwidth_out,
--                  last_keepalive_at, created_at

-- ============================================================================
-- STEP 2: Update sessions table
-- ============================================================================

-- Rename media_server_id to media_server_tag
ALTER TABLE sessions DROP COLUMN IF EXISTS media_server_id;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS media_server_tag VARCHAR(255);

-- Rename stream_id to stream_key
ALTER TABLE sessions DROP COLUMN IF EXISTS stream_id;
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS stream_key VARCHAR(255) NOT NULL DEFAULT '';

-- Remove device_id column
ALTER TABLE sessions DROP COLUMN IF EXISTS device_id;

-- Verify the sessions table structure
-- Expected columns: id, session_type, device_tag, channel_tag, user_id, state,
--                  client_ip, client_type, media_server_tag, stream_key, protocol,
--                  created_at, last_activity, expires_at, bytes_sent, bytes_received

-- ============================================================================
-- STEP 3: Verify changes
-- ============================================================================

-- Check streams columns
-- SELECT column_name FROM information_schema.columns WHERE table_name = 'streams' ORDER BY ordinal_position;

-- Check sessions columns  
-- SELECT column_name FROM information_schema.columns WHERE table_name = 'sessions' ORDER BY ordinal_position;
