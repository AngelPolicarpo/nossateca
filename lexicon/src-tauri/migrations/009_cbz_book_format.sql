-- Rebuild books table so CHECK constraint allows the cbz format.
CREATE TABLE IF NOT EXISTS books_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    author TEXT,
    format TEXT NOT NULL CHECK(format IN ('epub', 'pdf', 'mobi', 'cbz')),
    file_path TEXT NOT NULL UNIQUE,
    file_hash TEXT UNIQUE,
    status TEXT NOT NULL DEFAULT 'discovered',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO books_new (
    id, title, author, format, file_path, file_hash, status, created_at
) SELECT
    id, title, author, format, file_path, file_hash, status, created_at
FROM books;

DROP TABLE books;
ALTER TABLE books_new RENAME TO books;

CREATE INDEX IF NOT EXISTS idx_books_title ON books(title);
CREATE INDEX IF NOT EXISTS idx_books_author ON books(author);
CREATE INDEX IF NOT EXISTS idx_books_status ON books(status);
