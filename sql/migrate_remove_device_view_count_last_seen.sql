-- Migration: Remove view_count and last_seen from devices table
-- These fields belong to Stream, not Device
-- Date: 2026-06-13

BEGIN;

-- Remove view_count column from devices
ALTER TABLE devices DROP COLUMN IF EXISTS view_count;

-- Remove last_seen column from devices
ALTER TABLE devices DROP COLUMN IF EXISTS last_seen;

COMMIT;
