-- Create photos table
CREATE TABLE IF NOT EXISTS photo (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    hash TEXT UNIQUE NOT NULL,
    title TEXT,
    artist TEXT,
    copyright TEXT,
    notes TEXT,
    date_taken DATETIME,
    fullsize_path TEXT NOT NULL,
    websize_path TEXT NOT NULL,
    thumbnail_path TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for faster hash lookups (duplicate detection)
CREATE INDEX IF NOT EXISTS idx_photo_hash ON photo(hash);
