-- Add mat_preset column to photo table
ALTER TABLE photo ADD COLUMN mat_preset TEXT NOT NULL DEFAULT 'classic';
