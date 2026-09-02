-- Migration: Add device_groups table and group_id column to devices
-- Run this if device_groups table doesn't exist

CREATE TABLE IF NOT EXISTS device_groups (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    parent_id BIGINT REFERENCES device_groups(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_device_groups_parent ON device_groups(parent_id);

-- Add group_id column to devices if it doesn't exist
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'devices' AND column_name = 'group_id'
    ) THEN
        ALTER TABLE devices ADD COLUMN group_id BIGINT;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_devices_group_id ON devices(group_id);
