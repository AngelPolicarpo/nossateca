CREATE TABLE IF NOT EXISTS downloads (
    id TEXT PRIMARY KEY,
    source_url TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK(source_type IN ('http', 'torrent', 'opds')),
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

CREATE INDEX IF NOT EXISTS idx_downloads_status ON downloads(status);
CREATE INDEX IF NOT EXISTS idx_downloads_created ON downloads(created_at DESC);
