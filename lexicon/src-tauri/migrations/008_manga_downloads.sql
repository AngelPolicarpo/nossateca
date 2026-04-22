-- Rebuild downloads table so CHECK constraint allows the manga-cbz source_type.
CREATE TABLE IF NOT EXISTS downloads_new (
    id TEXT PRIMARY KEY,
    source_url TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('http', 'torrent', 'opds', 'manga-cbz')),
    file_name TEXT NOT NULL,
    file_path TEXT,
    status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN ('queued', 'downloading', 'paused', 'completed', 'failed', 'cancelled')),
    error_message TEXT,
    total_bytes INTEGER,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0,
    speed_bps INTEGER,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    torrent_info_hash TEXT,
    torrent_peers INTEGER,
    torrent_seeds INTEGER,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at DATETIME,
    completed_at DATETIME,
    book_id INTEGER REFERENCES books(id)
);

INSERT INTO downloads_new (
    id, source_url, source_type, file_name, file_path, status, error_message,
    total_bytes, downloaded_bytes, speed_bps, retry_count, max_retries,
    torrent_info_hash, torrent_peers, torrent_seeds,
    created_at, started_at, completed_at, book_id
) SELECT
    id, source_url, source_type, file_name, file_path, status, error_message,
    total_bytes, downloaded_bytes, speed_bps, retry_count, max_retries,
    torrent_info_hash, torrent_peers, torrent_seeds,
    created_at, started_at, completed_at, book_id
FROM downloads;

DROP TABLE downloads;
ALTER TABLE downloads_new RENAME TO downloads;

CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads(created_at DESC);
