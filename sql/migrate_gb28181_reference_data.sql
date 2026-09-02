-- GB28181 Reference Data Tables
-- Standard data from GB/T 28181-2016
-- Reference: https://www.twinkleway.com/gbcode/

BEGIN;

-- ============================================
-- GB28181 Device Type Codes
-- ============================================
CREATE TABLE IF NOT EXISTS gb_device_types (
    code VARCHAR(10) PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    name_en VARCHAR(100),
    category VARCHAR(50) NOT NULL,
    description TEXT,
    can_have_children BOOLEAN DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Frontend Main Devices (前端主设备) - Category 01
INSERT INTO gb_device_types (code, name, name_en, category, description, can_have_children, sort_order) VALUES
('111', '数字视频录像机', 'DVR', 'frontend_device', 'DVR', FALSE, 10),
('112', '视频服务器', 'Video Server', 'frontend_device', '视频服务器', FALSE, 11),
('113', '编码器', 'Encoder', 'frontend_device', '编码器', FALSE, 12),
('114', '解码器', 'Decoder', 'frontend_device', '解码器', FALSE, 13),
('115', '视频切换矩阵', 'Video Matrix', 'frontend_device', '视频切换矩阵', FALSE, 14),
('116', '音频切换矩阵', 'Audio Matrix', 'frontend_device', '音频切换矩阵', FALSE, 15),
('117', '报警控制器', 'Alarm Controller', 'frontend_device', '报警控制器', FALSE, 16),
('118', '网络视频录像机(NVR)', 'NVR', 'frontend_device', '网络视频录像机(NVR)', FALSE, 17),
('120', '在线视频图像信息采集系统', 'Online Video System', 'frontend_device', '在线视频图像信息采集系统', FALSE, 18),
('121', '视频卡口', 'Video Checkpoint', 'frontend_device', '视频卡口', FALSE, 19),
('122', '多目设备', 'Multi-Camera Device', 'frontend_device', '多目设备', FALSE, 20),
('123', '停车场出入口控制设备', 'Parking Gate', 'frontend_device', '停车场出入口控制设备', FALSE, 21),
('124', '人员出入口控制设备', 'Access Control', 'frontend_device', '人员出入口控制设备', FALSE, 22),
('125', '安检设备', 'Security Check', 'frontend_device', '安检设备', FALSE, 23),
('130', '混合硬盘录像机(HVR)', 'HVR', 'frontend_device', '混合硬盘录像机(HVR)', FALSE, 24)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    can_have_children = EXCLUDED.can_have_children,
    sort_order = EXCLUDED.sort_order;

-- Frontend Peripheral Devices (前端外围设备) - Category 02
INSERT INTO gb_device_types (code, name, name_en, category, description, can_have_children, sort_order) VALUES
('131', '摄像机', 'Camera', 'frontend_peripheral', '摄像机', FALSE, 30),
('132', '网络摄像机(IPC)', 'IPC', 'frontend_peripheral', '网络摄像机(IPC)', FALSE, 31),
('133', '显示器', 'Monitor', 'frontend_peripheral', '显示器', FALSE, 32),
('134', '报警输入设备(如红外、烟感、门禁等报警设备)', 'Alarm Input', 'frontend_peripheral', '报警输入设备(如红外、烟感、门禁等报警设备)', FALSE, 33),
('135', '报警输出设备(如警灯、警铃等设备)', 'Alarm Output', 'frontend_peripheral', '报警输出设备(如警灯、警铃等设备)', FALSE, 34),
('136', '语音输入设备', 'Voice Input', 'frontend_peripheral', '语音输入设备', FALSE, 35),
('137', '语音输出设备', 'Voice Output', 'frontend_peripheral', '语音输出设备', FALSE, 36),
('138', '移动传输设备', 'Mobile Transmit', 'frontend_peripheral', '移动传输设备', FALSE, 37),
('139', '其他外围设备', 'Other Peripheral', 'frontend_peripheral', '其他外围设备', FALSE, 38),
('140', '报警输出设备(如继电器或触发器控制的设备)', 'Alarm Relay', 'frontend_peripheral', '报警输出设备(如继电器或触发器控制的设备)', FALSE, 39),
('141', '道闸(控制车辆通行)', 'Barrier Gate', 'frontend_peripheral', '道闸(控制车辆通行)', FALSE, 40),
('142', '智能门(控制人员通行)', 'Smart Door', 'frontend_peripheral', '智能门(控制人员通行)', FALSE, 41),
('143', '凭证识别单元', 'Voucher Recognition', 'frontend_peripheral', '凭证识别单元', FALSE, 42)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    can_have_children = EXCLUDED.can_have_children,
    sort_order = EXCLUDED.sort_order;

-- Platform Devices (平台设备) - Category 03
INSERT INTO gb_device_types (code, name, name_en, category, description, can_have_children, sort_order) VALUES
('200', '中心信令控制服务器', 'Central Signaling', 'platform', '中心信令控制服务器', FALSE, 50),
('201', 'Web应用服务器', 'Web App Server', 'platform', 'Web应用服务器', FALSE, 51),
('202', '媒体分发服务器', 'Media Distribution', 'platform', '媒体分发服务器', FALSE, 52),
('203', '代理服务器', 'Proxy Server', 'platform', '代理服务器', FALSE, 53),
('204', '安全服务器', 'Security Server', 'platform', '安全服务器', FALSE, 54),
('205', '报警服务器', 'Alarm Server', 'platform', '报警服务器', FALSE, 55),
('206', '数据库服务器', 'Database Server', 'platform', '数据库服务器', FALSE, 56),
('207', 'GIS服务器', 'GIS Server', 'platform', 'GIS服务器', FALSE, 57),
('208', '管理服务器', 'Manager Server', 'platform', '管理服务器', FALSE, 58),
('209', '接入网关', 'Access Gateway', 'platform', '接入网关', FALSE, 59),
('210', '媒体存储服务器', 'Media Storage', 'platform', '媒体存储服务器', FALSE, 60),
('211', '信令安全路由网关', 'Signaling Gateway', 'platform', '信令安全路由网关', FALSE, 61),
('215', '业务分组', 'Business Group', 'platform', '业务分组', TRUE, 62),
('216', '虚拟组织', 'Virtual Organization', 'platform', '虚拟组织', FALSE, 63)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    can_have_children = EXCLUDED.can_have_children,
    sort_order = EXCLUDED.sort_order;

-- Center Users (中心用户) - Category 04
INSERT INTO gb_device_types (code, name, name_en, category, description, can_have_children, sort_order) VALUES
('300', '中心用户', 'Center User', 'center_user', '中心用户', FALSE, 70)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    can_have_children = EXCLUDED.can_have_children,
    sort_order = EXCLUDED.sort_order;

-- Terminal Users (终端用户) - Category 05
INSERT INTO gb_device_types (code, name, name_en, category, description, can_have_children, sort_order) VALUES
('400', '终端用户', 'Terminal User', 'terminal_user', '终端用户', FALSE, 80)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    can_have_children = EXCLUDED.can_have_children,
    sort_order = EXCLUDED.sort_order;

-- Platform External Servers (平台外接服务器) - Category 06
INSERT INTO gb_device_types (code, name, name_en, category, description, can_have_children, sort_order) VALUES
('500', '视频图像信息综合应用平台', 'Video Platform', 'platform_external', '视频图像信息综合应用平台', FALSE, 90),
('501', '视频图像信息运维管理平台', 'Video OPS', 'platform_external', '视频图像信息运维管理平台', FALSE, 91),
('502', '视频图像分析系统', 'Video Analytics', 'platform_external', '视频图像分析系统', FALSE, 92),
('503', '视频图像信息数据库', 'Video Database', 'platform_external', '视频图像信息数据库', FALSE, 93),
('505', '视频图像分析设备', 'Video Analysis Device', 'platform_external', '视频图像分析设备', FALSE, 94)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    category = EXCLUDED.category,
    description = EXCLUDED.description,
    can_have_children = EXCLUDED.can_have_children,
    sort_order = EXCLUDED.sort_order;

-- ============================================
-- GB28181 Industry Codes (行业编码) - GB/T 28181-2016
-- ============================================
CREATE TABLE IF NOT EXISTS gb_industry_codes (
    code VARCHAR(10) PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    name_en VARCHAR(100),
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO gb_industry_codes (code, name, name_en, description, sort_order) VALUES
('00', '社会治安路面接入', 'Social Security Road Access', '社会治安路面接入', 0),
('01', '社会治安社区接入', 'Social Security Community Access', '社会治安社区接入', 1),
('02', '社会治安内部接入', 'Social Security Internal Access', '社会治安内部接入', 2),
('03', '社会治安其他接入', 'Social Security Other Access', '社会治安其他接入', 3),
('04', '交通路面接入', 'Traffic Road Access', '交通路面接入', 4),
('05', '交通卡口接入', 'Traffic Checkpoint Access', '交通卡口接入', 5),
('06', '交通内部接入', 'Traffic Internal Access', '交通内部接入', 6),
('07', '交通其他接入', 'Traffic Other Access', '交通其他接入', 7),
('08', '城市管理接入', 'Urban Management Access', '城市管理接入', 8),
('09', '卫生环保接入', 'Health & Environment Access', '卫生环保接入', 9),
('10', '商检海关接入', 'Commerce & Customs Access', '商检海关接入', 10),
('11', '教育部门接入', 'Education Department Access', '教育部门接入', 11),
('12', '民航接入', 'Civil Aviation Access', '民航接入', 12),
('13', '铁路接入', 'Railway Access', '铁路接入', 13),
('14', '航运接入', 'Shipping Access', '航运接入', 14),
('40', '农、林、牧、渔业接入', 'Agriculture Access', '农、林、牧、渔业接入', 15),
('41', '采矿业接入', 'Mining Access', '采矿业接入', 16),
('42', '制造业接入', 'Manufacturing Access', '制造业接入', 17),
('43', '电力、热力、燃气及水生产和供应业接入', 'Utility Access', '电力、热力、燃气及水生产和供应业接入', 18),
('44', '建筑业接入', 'Construction Access', '建筑业接入', 19),
('45', '批发和零售业接入', 'Wholesale & Retail Access', '批发和零售业接入', 20),
('46', '交通运输、仓储和邮政业接入', 'Transport & Logistics Access', '交通运输、仓储和邮政业接入', 21),
('47', '住宿和餐饮业接入', 'Hospitality Access', '住宿和餐饮业接入', 22),
('48', '信息传输、软件和信息技术服务业接入', 'IT Services Access', '信息传输、软件和信息技术服务业接入', 23),
('49', '金融业接入', 'Finance Access', '金融业接入', 24),
('50', '房地产业接入', 'Real Estate Access', '房地产业接入', 25),
('51', '租赁和商务服务业接入', 'Leasing & Business Access', '租赁和商务服务业接入', 26),
('52', '科学研究和技术服务业接入', 'R&D Access', '科学研究和技术服务业接入', 27),
('53', '水利、环境和公共设施管理业接入', 'Water & Environment Access', '水利、环境和公共设施管理业接入', 28),
('54', '居民服务、修理和其他服务业接入', 'Residential Services Access', '居民服务、修理和其他服务业接入', 29),
('55', '教育接入', 'Education Access', '教育接入', 30),
('56', '卫生和社会工作接入', 'Health & Social Work Access', '卫生和社会工作接入', 31),
('57', '文化、体育和娱乐业接入', 'Culture & Sports Access', '文化、体育和娱乐业接入', 32),
('58', '公共管理、社会保障和社会组织接入', 'Public Administration Access', '公共管理、社会保障和社会组织接入', 33),
('59', '国际组织', 'International Organization', '国际组织', 34)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    description = EXCLUDED.description,
    sort_order = EXCLUDED.sort_order;

-- ============================================
-- GB28181 Network Identification Codes (网络标识码)
-- ============================================
CREATE TABLE IF NOT EXISTS gb_network_codes (
    code VARCHAR(10) PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    name_en VARCHAR(100),
    description TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO gb_network_codes (code, name, name_en, description, sort_order) VALUES
('0', '公安视频传输网0', 'Police Video Network 0', '公安视频传输网0', 0),
('1', '公安视频传输网1', 'Police Video Network 1', '公安视频传输网1', 1),
('2', '行业专网', 'Industry Private Network', '行业专网', 2),
('3', '政法信息网', 'Political & Legal Info Network', '政法信息网', 3),
('4', '公安移动信息网', 'Police Mobile Network', '公安移动信息网', 4),
('5', '公安信息网', 'Police Info Network', '公安信息网', 5),
('6', '电子政务外网', 'Government Network', '电子政务外网', 6),
('7', '互联网等公共网络', 'Public Network', '互联网等公共网络', 7),
('8', '专线', 'Dedicated Line', '专线', 8)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    name_en = EXCLUDED.name_en,
    description = EXCLUDED.description,
    sort_order = EXCLUDED.sort_order;

-- ============================================
-- Indexes
-- ============================================
CREATE INDEX IF NOT EXISTS idx_gb_device_types_category ON gb_device_types(category);
CREATE INDEX IF NOT EXISTS idx_gb_device_types_sort ON gb_device_types(sort_order);
CREATE INDEX IF NOT EXISTS idx_gb_industry_codes_sort ON gb_industry_codes(sort_order);
CREATE INDEX IF NOT EXISTS idx_gb_network_codes_sort ON gb_network_codes(sort_order);

COMMIT;
