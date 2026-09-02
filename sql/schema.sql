-- RustCam Database Schema (Alternative/Fresh Install)
-- PostgreSQL 14+
-- Run this script to initialize database tables

BEGIN;

-- ============================================
-- Users Table
-- ============================================
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    role VARCHAR(50) NOT NULL DEFAULT 'User',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

-- ============================================
-- Device Groups Table
-- ============================================
CREATE TABLE IF NOT EXISTS device_groups (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    parent_id BIGINT REFERENCES device_groups(id) ON DELETE CASCADE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_device_groups_parent ON device_groups(parent_id);

-- ============================================
-- Devices Table (Unified Device Model)
-- Supports both Devices (NVR, IPC, DVR) and Channels
-- Distinguish by: is_channel = true/false
-- ============================================
CREATE TABLE IF NOT EXISTS devices (
    id BIGSERIAL PRIMARY KEY,
    device_tag VARCHAR(50) NOT NULL UNIQUE,    -- Global unique identifier
    name VARCHAR(255) NOT NULL,
    protocol VARCHAR(50) NOT NULL,             -- ONVIF / GB28181 / RTSP
    
    -- Hierarchy
    parent_device_tag VARCHAR(50),            -- Parent device's device_tag
    is_channel BOOLEAN DEFAULT FALSE,          -- Core: is this a channel?
    
    -- Device Type
    device_type VARCHAR(50) DEFAULT 'Other', -- NVR / IPC / DVR / Camera / Platform / Other
    device_type_code VARCHAR(10),             -- GB28181 raw type code (111/112/118/131/132)
    
    -- Device Info (used when is_channel = false)
    host VARCHAR(255) DEFAULT '',
    port INTEGER DEFAULT 0,
    device_username VARCHAR(255),
    device_password VARCHAR(255),
    playback_username VARCHAR(255),
    playback_password VARCHAR(255),
    
    -- Channel Info (used when is_channel = true)
    channel_id VARCHAR(50),                   -- Protocol channel identifier
    channel_extended JSONB DEFAULT '{}',      -- Channel attributes (baud_rate, resolution, codec, fps)
    is_default BOOLEAN DEFAULT FALSE,         -- Default channel for playback
    
    -- Common
    status VARCHAR(50) NOT NULL DEFAULT 'offline',
    app VARCHAR(255),
    media_server_tag VARCHAR(255),
    region_code VARCHAR(100),
    is_public BOOLEAN NOT NULL DEFAULT FALSE,
    group_id BIGINT REFERENCES device_groups(id) ON DELETE SET NULL,
    extended JSONB DEFAULT '{}',
    push_urls JSONB DEFAULT '[]',
    pull_urls JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_devices_status ON devices(status);
CREATE INDEX IF NOT EXISTS idx_devices_group_id ON devices(group_id);
CREATE INDEX IF NOT EXISTS idx_devices_is_channel ON devices(is_channel);
CREATE INDEX IF NOT EXISTS idx_devices_device_type ON devices(device_type);
CREATE INDEX IF NOT EXISTS idx_devices_channel_id ON devices(channel_id);
CREATE INDEX IF NOT EXISTS idx_devices_parent_tag ON devices(parent_device_tag);
CREATE INDEX IF NOT EXISTS idx_devices_is_default ON devices(is_default);
CREATE INDEX IF NOT EXISTS idx_devices_channel_extended ON devices USING GIN (channel_extended);

-- ============================================
-- Servers Table (Media Servers)
-- ============================================
CREATE TABLE IF NOT EXISTS servers (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL UNIQUE,
    url TEXT NOT NULL,
    api_key TEXT NOT NULL,
    server_type VARCHAR(50) NOT NULL,
    weight INTEGER NOT NULL DEFAULT 100,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    server_tag VARCHAR(255) NOT NULL DEFAULT '',
    protocol_ports JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_servers_type ON servers(server_type);
CREATE INDEX IF NOT EXISTS idx_servers_enabled ON servers(enabled);

-- ============================================
-- Streams Table
-- ============================================
CREATE TABLE IF NOT EXISTS streams (
    id BIGSERIAL PRIMARY KEY,
    device_tag VARCHAR(255),
    channel_tag VARCHAR(255),
    media_server_tag VARCHAR(255) NOT NULL DEFAULT '',
    app VARCHAR(255) NOT NULL DEFAULT '',
    token VARCHAR(255) NOT NULL DEFAULT '',
    state VARCHAR(50) NOT NULL DEFAULT 'Idle',
    retry_count SMALLINT NOT NULL DEFAULT 0,
    max_retries SMALLINT NOT NULL DEFAULT 20,
    last_error TEXT,
    viewer_count INTEGER NOT NULL DEFAULT 0,
    bandwidth_in BIGINT NOT NULL DEFAULT 0,
    bandwidth_out BIGINT NOT NULL DEFAULT 0,
    last_keepalive_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_streams_device_tag ON streams(device_tag);
CREATE INDEX IF NOT EXISTS idx_streams_state ON streams(state);

-- ============================================
-- Sessions Table
-- ============================================
CREATE TABLE IF NOT EXISTS sessions (
    id BIGSERIAL PRIMARY KEY,
    session_type VARCHAR(50) NOT NULL,
    user_id BIGINT NOT NULL,
    state VARCHAR(50) NOT NULL DEFAULT 'initializing',
    client_ip VARCHAR(255),
    client_type VARCHAR(100),
    media_server_tag VARCHAR(255),
    protocol VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    bytes_sent BIGINT NOT NULL DEFAULT 0,
    bytes_received BIGINT NOT NULL DEFAULT 0,
    device_tag VARCHAR(255),
    channel_tag VARCHAR(255)
);
CREATE INDEX IF NOT EXISTS idx_sessions_device_tag ON sessions(device_tag);
CREATE INDEX IF NOT EXISTS idx_sessions_state ON sessions(state);

-- ============================================
-- Recordings Table
-- ============================================
CREATE TABLE IF NOT EXISTS recordings (
    id BIGSERIAL PRIMARY KEY,
    device_tag VARCHAR(255),
    channel_tag VARCHAR(255),
    media_server_name VARCHAR(255) NOT NULL,
    state VARCHAR(50) NOT NULL DEFAULT 'Starting',
    format VARCHAR(50) NOT NULL DEFAULT 'MP4',
    output_path TEXT,
    file_size BIGINT NOT NULL DEFAULT 0,
    duration_secs BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    stopped_at TIMESTAMPTZ,
    error_message TEXT,
    labels TEXT,
    filename TEXT
);
CREATE INDEX IF NOT EXISTS idx_recordings_device_tag ON recordings(device_tag);
CREATE INDEX IF NOT EXISTS idx_recordings_state ON recordings(state);

-- ============================================
-- Regions Table
-- ============================================
CREATE TABLE IF NOT EXISTS regions (
    code VARCHAR(100) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    level SMALLINT NOT NULL DEFAULT 0,
    parent_code VARCHAR(100) REFERENCES regions(code) ON DELETE CASCADE,
    province_name VARCHAR(255),
    city_name VARCHAR(255),
    district_name VARCHAR(255),
    gb28181_code VARCHAR(100) NOT NULL DEFAULT '',
    device_count INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_regions_parent ON regions(parent_code);

-- ============================================
-- PTZ Presets Table
-- ============================================
CREATE TABLE IF NOT EXISTS ptz_presets (
    id BIGSERIAL PRIMARY KEY,
    device_id BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    token VARCHAR(255) NOT NULL,
    position_pan DOUBLE PRECISION,
    position_tilt DOUBLE PRECISION,
    position_zoom DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(device_id, token)
);
CREATE INDEX IF NOT EXISTS idx_ptz_presets_device_id ON ptz_presets(device_id);

-- ============================================
-- PTZ Control Logs Table
-- ============================================
CREATE TABLE IF NOT EXISTS ptz_control_logs (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT,
    device_id BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    command VARCHAR(100) NOT NULL,
    speed SMALLINT NOT NULL DEFAULT 50,
    result VARCHAR(50) NOT NULL DEFAULT 'success',
    error_message TEXT,
    call_id VARCHAR(100),
    sip_code INTEGER,
    device_response TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_ptz_logs_device_id ON ptz_control_logs(device_id);
CREATE INDEX IF NOT EXISTS idx_ptz_logs_created_at ON ptz_control_logs(created_at DESC);

-- ============================================
-- Player Window Layouts Table
-- ============================================
CREATE TABLE IF NOT EXISTS player_window_layouts (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    rows INTEGER NOT NULL DEFAULT 2,
    cols INTEGER NOT NULL DEFAULT 2,
    layout_json JSONB NOT NULL DEFAULT '[]',
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_player_layouts_name ON player_window_layouts(name);

-- Insert default layout
INSERT INTO player_window_layouts (name, rows, cols, layout_json, is_default)
VALUES (
    'Default 2x2',
    2, 2,
    '[{"id":"1","row":0,"col":0,"row_span":1,"col_span":1,"label":null},{"id":"2","row":0,"col":1,"row_span":1,"col_span":1,"label":null},{"id":"3","row":1,"col":0,"row_span":1,"col_span":1,"label":null},{"id":"4","row":1,"col":1,"row_span":1,"col_span":1,"label":null}]',
    TRUE
) ON CONFLICT DO NOTHING;

-- ============================================
-- Alarms Table (通用告警表，支持多协议)
-- ============================================
CREATE TABLE IF NOT EXISTS alarms (
    id BIGSERIAL PRIMARY KEY,
    device_id BIGINT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    device_tag VARCHAR(64) NOT NULL,
    alarm_type VARCHAR(64) NOT NULL DEFAULT 'Alarm',
    alarm_time TIMESTAMP NOT NULL,
    alarm_method INT DEFAULT 1,
    alarm_priority INT DEFAULT 0,
    description TEXT,
    processed BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_alarms_device_time ON alarms(device_id, alarm_time DESC);
CREATE INDEX IF NOT EXISTS idx_alarms_processed ON alarms(processed) WHERE NOT processed;

COMMIT;
