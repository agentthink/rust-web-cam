#!/usr/bin/env python3
"""
Migration script to add unified device model fields to the database.
Adds: is_channel, device_type, device_type_code, channel_id

Usage: python migrate_unified_device_model.py
"""

import psycopg2
import sys
import re
from urllib.parse import urlparse

CONFIG_PATH = "../config.toml"


def parse_config():
    """Parse database URL from config.toml"""
    with open(CONFIG_PATH, "r") as f:
        content = f.read()

    # Find database URL
    match = re.search(r'url\s*=\s*["\']([^"\']+)["\']', content)
    if not match:
        raise ValueError("Could not find database URL in config.toml")

    return match.group(1)


def get_connection(db_url):
    """Create database connection"""
    parsed = urlparse(db_url)

    conn = psycopg2.connect(
        host=parsed.hostname or "localhost",
        port=parsed.port or 5432,
        database=parsed.path.lstrip("/") or "rustcam",
        user=parsed.username or "postgres",
        password=parsed.password or "",
    )
    return conn


def check_column_exists(conn, table, column):
    """Check if column exists in table"""
    cursor = conn.cursor()
    cursor.execute(
        """
        SELECT column_name 
        FROM information_schema.columns 
        WHERE table_name = %s AND column_name = %s
    """,
        (table, column),
    )
    result = cursor.fetchone()
    cursor.close()
    return result is not None


def add_column(conn, table, column, definition):
    """Add a column to a table"""
    if not check_column_exists(conn, table, column):
        cursor = conn.cursor()
        cursor.execute(f"ALTER TABLE {table} ADD COLUMN {column} {definition}")
        conn.commit()
        print(f"  Added column: {table}.{column}")
        cursor.close()
    else:
        print(f"  Column exists: {table}.{column}")


def parse_device_type_from_gb28181(device_tag):
    """Parse device type from GB28181 20-digit ID"""
    if not device_tag or len(device_tag) != 20 or not device_tag.isdigit():
        return "Other", None

    # Position 11-13 (0-indexed: 10-12) is the type code
    type_code = device_tag[10:13]

    type_mapping = {
        "111": ("DVR", type_code),
        "112": ("VideoServer", type_code),
        "113": ("Encoder", type_code),
        "118": ("NVR", type_code),
        "130": ("DVR", type_code),  # HVR
        "131": ("Camera", type_code),
        "132": ("IPC", type_code),
    }

    # Check if platform type (200-216)
    if type_code.startswith("2") or type_code in ("215", "216"):
        return ("Platform", type_code)

    return type_mapping.get(type_code, ("Other", type_code))


def migrate_unified_device_model():
    """Run the unified device model migration"""
    print("=" * 60)
    print("Unified Device Model Migration")
    print("=" * 60)

    # Get database URL from config
    db_url = parse_config()
    print(f"\nDatabase: {db_url.split('@')[-1] if '@' in db_url else db_url}")

    conn = get_connection(db_url)

    try:
        # 1. Add new columns
        print("\n[1/4] Adding new columns to devices table...")
        add_column(conn, "devices", "is_channel", "BOOLEAN DEFAULT FALSE")
        add_column(conn, "devices", "device_type", "VARCHAR(50) DEFAULT 'Other'")
        add_column(conn, "devices", "device_type_code", "VARCHAR(10)")
        add_column(conn, "devices", "channel_id", "VARCHAR(50)")

        # 2. Add indexes
        print("\n[2/4] Adding indexes...")
        indexes = [
            ("idx_devices_is_channel", "devices", "is_channel"),
            ("idx_devices_device_type", "devices", "device_type"),
            ("idx_devices_channel_id", "devices", "channel_id"),
            ("idx_devices_parent_tag", "devices", "parent_device_tag"),
        ]

        cursor = conn.cursor()
        for idx_name, table, column in indexes:
            cursor.execute(f"""
                CREATE INDEX IF NOT EXISTS {idx_name} ON {table}({column})
            """)
            print(f"  Index: {idx_name}")
        conn.commit()
        cursor.close()

        # 3. Backfill device_type from GB28181 device_tag
        print("\n[3/4] Backfilling device_type from GB28181 device_tag...")
        cursor = conn.cursor()

        # Get all devices with GB28181-like device_tags (20 digits)
        cursor.execute("""
            SELECT id, device_tag FROM devices 
            WHERE device_tag ~ '^[0-9]{20}$'
        """)
        devices = cursor.fetchall()

        for device_id, device_tag in devices:
            device_type, type_code = parse_device_type_from_gb28181(device_tag)
            cursor.execute(
                """
                UPDATE devices 
                SET device_type = %s, device_type_code = %s
                WHERE id = %s
            """,
                (device_type, type_code, device_id),
            )

        conn.commit()
        print(f"  Updated {len(devices)} GB28181 devices")
        cursor.close()

        # 4. Mark channels and devices
        print("\n[4/4] Marking is_channel and channel_id...")
        cursor = conn.cursor()

        # Mark devices with streams as channels (channel_id = device_tag)
        cursor.execute("""
            UPDATE devices d
            SET is_channel = TRUE, channel_id = d.device_tag
            WHERE EXISTS (
                SELECT 1 FROM streams s WHERE s.device_id = d.id
            )
        """)
        channels_updated = cursor.rowcount

        # Mark devices with parent_device_tag but no streams as sub-devices (IPC under NVR)
        cursor.execute("""
            UPDATE devices
            SET is_channel = FALSE
            WHERE parent_device_tag IS NOT NULL
            AND is_channel = FALSE
        """)
        subdev_updated = cursor.rowcount

        conn.commit()
        print(f"  Marked {channels_updated} channels (have streams)")
        print(f"  Marked {subdev_updated} sub-devices (parent_device_tag set)")

        cursor.close()

        print("\n" + "=" * 60)
        print("Migration completed successfully!")
        print("=" * 60)

        # Show summary
        print("\n[Summary]")
        cursor = conn.cursor()
        cursor.execute("""
            SELECT is_channel, device_type, COUNT(*) 
            FROM devices 
            GROUP BY is_channel, device_type 
            ORDER BY is_channel, device_type
        """)
        print("\nDevice counts by type:")
        print(f"{'is_channel':<12} {'device_type':<20} {'count':<10}")
        print("-" * 42)
        for row in cursor.fetchall():
            print(f"{str(row[0]):<12} {row[1]:<20} {row[2]:<10}")
        cursor.close()

    finally:
        conn.close()


if __name__ == "__main__":
    try:
        migrate_unified_device_model()
    except Exception as e:
        print(f"\nError: {e}")
        sys.exit(1)
