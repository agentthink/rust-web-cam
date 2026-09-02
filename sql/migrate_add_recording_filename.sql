-- Migration: Add filename column to recordings table
-- This column was added to the domain model but was missing from some database instances

ALTER TABLE recordings ADD COLUMN IF NOT EXISTS filename TEXT;

-- Also ensure other columns that might be missing
ALTER TABLE recordings ADD COLUMN IF NOT EXISTS labels TEXT;
