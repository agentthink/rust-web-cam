ALTER TABLE ptz_control_logs ADD COLUMN IF NOT EXISTS call_id VARCHAR(100);
ALTER TABLE ptz_control_logs ADD COLUMN IF NOT EXISTS sip_code INTEGER;
ALTER TABLE ptz_control_logs ADD COLUMN IF NOT EXISTS device_response TEXT;
