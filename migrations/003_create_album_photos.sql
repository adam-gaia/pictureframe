-- Junction table for many-to-many relationship between albums and photos
CREATE TABLE IF NOT EXISTS album_photo (
    album_id INTEGER NOT NULL,
    photo_id INTEGER NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (album_id, photo_id),
    FOREIGN KEY (album_id) REFERENCES album(id) ON DELETE CASCADE,
    FOREIGN KEY (photo_id) REFERENCES photo(id) ON DELETE CASCADE
);

-- Index for efficient album photo lookups ordered by position
CREATE INDEX IF NOT EXISTS idx_album_photo_position ON album_photo(album_id, position);
