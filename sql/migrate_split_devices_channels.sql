-- Migrate to separate devices and channels tables
-- This migration:
-- 1. Creates a new channels table with (device_tag, channel_tag) as unique key
-- 2. Migrates channel data from devices table
-- 3. Removes channel-specific fields from devices table (but keeps parent_device_tag for device hierarchy)

-- ============================================
-- Step 1: Create channels table
-- ============================================
CREATE TABLE IF NOT EXISTS channels (
    id BIGSERIAL PRIMARY KEY,
    device_tag VARCHAR(50) NOT NULL,                    -- FK to devices.device_tag
    channel_tag VARCHAR(50) NOT NULL,                   -- Protocol channel identifier (e.g., GB28181 20-digit ID)
    
    -- Channel Identity
    name VARCHAR(255) NOT NULL DEFAULT '',
    status VARCHAR(50) NOT NULL DEFAULT 'offline',
    
    -- GB28181 Specific
    device_type VARCHAR(50) DEFAULT 'Other',            -- IPC / Camera / etc
    device_type_code VARCHAR(10),                       -- GB28181 raw type code (131/132/135/136/137)
    
    -- Channel Extended
    channel_extended JSONB DEFAULT '{}',                -- Channel attributes (resolution, codec, fps, ptz_type, etc.)
    is_default BOOLEAN DEFAULT FALSE,                  -- Default channel for playback
    
    -- Hierarchy (for NVR channels or nested groups - mirrors GB28181 parental)
    parent_channel_tag VARCHAR(50),                     -- Parent channel's channel_tag (for channel hierarchy)
    
    -- Location Info (from GB28181 Catalog)
    civil_code VARCHAR(100),
    address VARCHAR(500),
    ip_address VARCHAR(50),
    port INTEGER DEFAULT 0,
    
    -- Device Info
    manufacturer VARCHAR(255),
    model VARCHAR(255),
    
    -- Parental / Grouping (from GB28181 Catalog)
    parental INTEGER DEFAULT 0,                          -- 0 = normal channel, >0 = has sub-devices/groups
    
    -- Reference Data
    extended JSONB DEFAULT '{}',
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    -- Constraints
    UNIQUE (device_tag, channel_tag)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_channels_device_tag ON channels(device_tag);
CREATE INDEX IF NOT EXISTS idx_channels_channel_tag ON channels(channel_tag);
CREATE INDEX IF NOT EXISTS idx_channels_parent_channel ON channels(parent_channel_tag);
CREATE INDEX IF NOT EXISTS idx_channels_status ON channels(status);
CREATE INDEX IF NOT EXISTS idx_channels_is_default ON channels(is_default);
CREATE INDEX IF NOT EXISTS idx_channels_channel_extended ON channels USING GIN (channel_extended);

-- ============================================
-- Step 2: Migrate channel data from devices table
-- ============================================
INSERT INTO channels (
    device_tag,
    channel_tag,
    name,
    status,
    device_type,
    device_type_code,
    channel_extended,
    is_default,
    parent_channel_tag,
    civil_code,
    address,
    ip_address,
    port,
    manufacturer,
    model,
    parental,
    extended,
    created_at
)
SELECT 
    parent_device_tag AS device_tag,                    -- parent_device_tag is the device this channel belongs to
    device_tag AS channel_tag,                         -- device_tag becomes channel_tag
    name,
    status,
    device_type,
    device_type_code,
    COALESCE(channel_extended, '{}'),
    COALESCE(is_default, FALSE),
    NULL,                                              -- parent_channel_tag (not in old schema)
    extended->>'civil_code',
    extended->>'address',
    extended->>'ip_address',
    port,
    extended->>'manufacturer',
    extended->>'model',
    0,                                                -- parental (default)
    extended,
    created_at
FROM devices 
WHERE is_channel = TRUE 
  AND parent_device_tag IS NOT NULL
  AND device_tag IS NOT NULL;

-- ============================================
-- Step 3: Remove channel-specific columns from devices
-- Note: parent_device_tag is KEPT for device hierarchy (e.g., NVR -> sub-devices/platforms)
-- ============================================
ALTER TABLE devices 
    DROP COLUMN IF EXISTS is_channel,
    DROP COLUMN IF EXISTS channel_id,
    DROP COLUMN IF EXISTS channel_extended,
    DROP COLUMN IF EXISTS is_default;

-- ============================================
-- Step 4: Add channel_count to devices (for NVR/counting channels)
-- ============================================
ALTER TABLE devices ADD COLUMN IF NOT EXISTS channel_count INTEGER DEFAULT 0;

-- ============================================
-- Step 5: Update channel_count based on migrated data
-- ============================================
UPDATE devices d SET channel_count = (
    SELECT COUNT(*) FROM channels c WHERE c.device_tag = d.device_tag
) WHERE d.device_tag IN (SELECT DISTINCT device_tag FROM channels);

-- ============================================
-- Step 6: Verify migration
-- ============================================
-- SELECT 'Channels created:' AS info, COUNT(*) AS count FROM channels;
-- SELECT 'Devices with channels:' AS info, COUNT(*) AS count FROM devices WHERE channel_count > 0;
