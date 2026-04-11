use anyhow::Result;
use sqlx::SqlitePool;

use crate::models::{Annotation, NewAnnotation};

pub struct AnnotationRepository<'a> {
    pool: &'a SqlitePool,
}

impl<'a> AnnotationRepository<'a> {
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        id: &str,
        book_id: i64,
        annotation: &NewAnnotation,
    ) -> Result<Annotation> {
        let color = annotation
            .color
            .clone()
            .unwrap_or_else(|| "yellow".to_string());

        sqlx::query(
            "INSERT INTO annotations (id, book_id, type, position, position_end, selected_text, note_text, color) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(book_id)
        .bind(&annotation.annotation_type)
        .bind(&annotation.position)
        .bind(&annotation.position_end)
        .bind(&annotation.selected_text)
        .bind(&annotation.note_text)
        .bind(&color)
        .execute(self.pool)
        .await?;

        let row = sqlx::query_as::<_, Annotation>(
            "SELECT id, book_id, type as annotation_type, position, position_end, selected_text, note_text, color, created_at, updated_at FROM annotations WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_one(self.pool)
        .await?;

        Ok(row)
    }

    pub async fn list_by_book(&self, book_id: i64) -> Result<Vec<Annotation>> {
        let rows = sqlx::query_as::<_, Annotation>(
            "SELECT id, book_id, type as annotation_type, position, position_end, selected_text, note_text, color, created_at, updated_at FROM annotations WHERE book_id = ? ORDER BY created_at DESC",
        )
        .bind(book_id)
        .fetch_all(self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn update_note(&self, id: &str, note_text: &str) -> Result<()> {
        sqlx::query(
            "UPDATE annotations SET note_text = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(note_text)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_color(&self, id: &str, color: &str) -> Result<()> {
        sqlx::query(
            "UPDATE annotations SET color = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(color)
        .bind(id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM annotations WHERE id = ?")
            .bind(id)
            .execute(self.pool)
            .await?;

        Ok(())
    }
}
