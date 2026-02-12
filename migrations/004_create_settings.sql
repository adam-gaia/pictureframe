-- Settings table (single row for app settings)
CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    current_album_id INTEGER,
    current_photo_index INTEGER NOT NULL DEFAULT 0,
    interval_seconds INTEGER NOT NULL DEFAULT 180,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (current_album_id) REFERENCES album(id) ON DELETE SET NULL
);

-- Insert default settings row
INSERT OR IGNORE INTO settings (id, interval_seconds) VALUES (1, 180);
