-- Migration: Add Unified Device Model Fields
-- Adds: is_channel, device_type, device_type_code, channel_id

BEGIN;

-- Add new columns to devices table
ALTER TABLE devices ADD COLUMN IF NOT EXISTS is_channel BOOLEAN DEFAULT FALSE;
ALTER TABLE devices ADD COLUMN IF NOT EXISTS device_type VARCHAR(50) DEFAULT 'Other';
ALTER TABLE devices ADD COLUMN IF NOT EXISTS device_type_code VARCHAR(10);
ALTER TABLE devices ADD COLUMN IF NOT EXISTS channel_id VARCHAR(50);

-- Add indexes for new fields
CREATE INDEX IF NOT EXISTS idx_devices_is_channel ON devices(is_channel);
CREATE INDEX IF NOT EXISTS idx_devices_device_type ON devices(device_type);
CREATE INDEX IF NOT EXISTS idx_devices_channel_id ON devices(channel_id);
CREATE INDEX IF NOT EXISTS idx_devices_parent_tag ON devices(parent_device_tag);

-- Backfill device_type based on existing data
-- For GB28181 devices, parse from device_tag (positions 11-13)
UPDATE devices 
SET device_type = 
    CASE 
        WHEN device_tag ~ '^\d{20}$' THEN
            CASE 
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '111' THEN 'DVR'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '112' THEN 'VideoServer'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '113' THEN 'Encoder'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '118' THEN 'NVR'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '130' THEN 'DVR'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '131' THEN 'Camera'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) = '132' THEN 'IPC'
                WHEN SUBSTRING(device_tag FROM 11 FOR 3) IN ('200','201','202','203','204','205','206','207','208','209','210','211','215','216') THEN 'Platform'
                ELSE 'Other'
            END
        ELSE 'Other'
    END
WHERE device_type = 'Other' OR device_type IS NULL;

-- Mark existing devices with parent_device_tag as non-channel devices (they are parent devices like NVR/IPC)
-- The actual channels are identified by having parent_device_tag set AND being in streams
UPDATE devices SET is_channel = false WHERE parent_device_tag IS NULL;

-- For devices with parent_device_tag, mark as non-channel (these are sub-devices like IPC under NVR)
UPDATE devices SET is_channel = false WHERE parent_device_tag IS NOT NULL;

-- Set channel_id for devices that have streams pointing to them
-- This indicates they are the actual streaming units
UPDATE devices d
SET channel_id = d.device_tag
WHERE EXISTS (
    SELECT 1 FROM streams s WHERE s.device_id = d.id
);

-- Add comments
COMMENT ON COLUMN devices.is_channel IS 'Whether this device is a channel (true) or a device/unit (false)';
COMMENT ON COLUMN devices.device_type IS 'Device type: NVR, IPC, DVR, Camera, Platform, Other';
COMMENT ON COLUMN devices.device_type_code IS 'GB28181 raw type code (111/112/118/131/132/etc)';
COMMENT ON COLUMN devices.channel_id IS 'Protocol channel identifier: GB28181 channel ID or ONVIF profile token';

COMMIT;
