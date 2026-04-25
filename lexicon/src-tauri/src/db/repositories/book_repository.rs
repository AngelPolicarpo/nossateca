use anyhow::{bail, Result};
use sqlx::SqlitePool;

use crate::models::Book;
use crate::storage;

pub struct BookRepository<'a> {
    pool: &'a SqlitePool,
}

fn expand_book_path(book: &mut Book) {
    let trimmed = book.file_path.trim();
    if trimmed.is_empty() {
        return;
    }
    let absolute = storage::expand_stored_path(trimmed);
    book.file_path = absolute.to_string_lossy().to_string();
}

impl<'a> BookRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, book: &Book) -> Result<()> {
        sqlx::query(
            "INSERT INTO books (title, author, format, file_path, file_hash, status) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&book.title)
        .bind(&book.author)
        .bind(&book.format)
        .bind(&book.file_path)
        .bind(&book.file_hash)
        .bind(&book.status)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn find_by_hash(&self, hash: &str) -> Result<Option<Book>> {
        let row = sqlx::query_as::<_, Book>(
            "SELECT id, title, author, format, file_path, file_hash, status, created_at FROM books WHERE file_hash = ? LIMIT 1",
        )
        .bind(hash)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|mut book| {
            expand_book_path(&mut book);
            book
        }))
    }

    pub async fn find_by_id(&self, book_id: i64) -> Result<Option<Book>> {
        let row = sqlx::query_as::<_, Book>(
            "SELECT id, title, author, format, file_path, file_hash, status, created_at FROM books WHERE id = ? LIMIT 1",
        )
        .bind(book_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row.map(|mut book| {
            expand_book_path(&mut book);
            book
        }))
    }

    pub async fn list_all(&self) -> Result<Vec<Book>> {
        let mut rows = sqlx::query_as::<_, Book>(
            "SELECT id, title, author, format, file_path, file_hash, status, created_at FROM books ORDER BY created_at DESC, id DESC",
        )
        .fetch_all(self.pool)
        .await?;

        for book in rows.iter_mut() {
            expand_book_path(book);
        }

        Ok(rows)
    }

    pub async fn delete_by_id(&self, book_id: i64) -> Result<()> {
        let result = sqlx::query("DELETE FROM books WHERE id = ?")
            .bind(book_id)
            .execute(self.pool)
            .await?;

        if result.rows_affected() == 0 {
            bail!("Book not found");
        }

        Ok(())
    }

    pub async fn update_status(&self, book_id: i64, status: &str) -> Result<()> {
        let result = sqlx::query("UPDATE books SET status = ? WHERE id = ?")
            .bind(status)
            .bind(book_id)
            .execute(self.pool)
            .await?;

        if result.rows_affected() == 0 {
            bail!("Book not found");
        }

        Ok(())
    }
}
