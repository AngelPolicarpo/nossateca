use anyhow::Result;
use sqlx::SqlitePool;
use tokio::task;

use crate::ai::{EmbeddingEngine, LlmEngine};

#[derive(Debug, Clone)]
pub struct RagQueryResult {
    pub answer: String,
    pub source_level: String,
    pub source_label: String,
}

#[derive(Debug, Clone)]
struct RankedChunk {
    chunk_id: i64,
    chapter_index: i64,
    char_start: i64,
    chunk_text: String,
    similarity: f32,
    rerank_score: f32,
}

pub struct RagEngine<'a> {
    pub embeddings: EmbeddingEngine,
    pub llm: LlmEngine,
    pool: &'a SqlitePool,
}

impl<'a> RagEngine<'a> {
    pub fn new(pool: &'a SqlitePool, embeddings: EmbeddingEngine, llm: LlmEngine) -> Self {
        Self {
            embeddings,
            llm,
            pool,
        }
    }

    pub async fn query(&self, book_id: &str, question: &str) -> Result<RagQueryResult> {
        let query_embedding = self.embeddings.embed_query(question)?;
        let parsed_book_id = book_id.parse::<i64>()?;
        let question_lower = question.to_ascii_lowercase();
        let requested_chapter = extract_requested_chapter(question);

        if is_book_overview_question(&question_lower) {
            if let Some((summary_text,)) = sqlx::query_as::<_, (String,)>(
                "SELECT summary_text FROM book_summaries WHERE book_id = ? LIMIT 1",
            )
            .bind(parsed_book_id)
            .fetch_optional(self.pool)
            .await?
            {
                let answer = self
                    .generate_from_summary(question, &summary_text, "livro")
                    .await?;

                return Ok(RagQueryResult {
                    answer,
                    source_level: "level3".to_string(),
                    source_label: "Visão geral do livro".to_string(),
                });
            }
        }

        if let Some(chapter_number) = requested_chapter {
            let chapter_index = chapter_number.saturating_sub(1) as i64;
            if let Some((summary_text,)) = sqlx::query_as::<_, (String,)>(
                "SELECT summary_text FROM chapter_summaries WHERE book_id = ? AND chapter_index = ? LIMIT 1",
            )
            .bind(parsed_book_id)
            .bind(chapter_index)
            .fetch_optional(self.pool)
            .await?
            {
                let answer = self
                    .generate_from_summary(question, &summary_text, &format!("capítulo {}", chapter_number))
                    .await?;

                return Ok(RagQueryResult {
                    answer,
                    source_level: "level2".to_string(),
                    source_label: format!("Resumo do Capítulo {}", chapter_number),
                });
            }

            let chapter_chunk_rows = sqlx::query_as::<_, (String,)>(
                "SELECT chunk_text FROM book_chunks
                 WHERE book_id = ? AND chapter_index = ?
                 ORDER BY char_start ASC
                 LIMIT 12",
            )
            .bind(parsed_book_id)
            .bind(chapter_index)
            .fetch_all(self.pool)
            .await?;

            let chapter_chunks = chapter_chunk_rows
                .into_iter()
                .map(|(chunk_text,)| chunk_text)
                .collect::<Vec<String>>();

            if !chapter_chunks.is_empty() {
                let answer = self
                    .generate_from_chapter_chunks(question, chapter_number, &chapter_chunks)
                    .await?;

                return Ok(RagQueryResult {
                    answer,
                    source_level: "level1".to_string(),
                    source_label: format!("Trechos do Capítulo {}", chapter_number),
                });
            }

            let (indexed_chapter_count, max_chapter_index) = sqlx::query_as::<_, (i64, Option<i64>)>(
                "SELECT COUNT(DISTINCT chapter_index), MAX(chapter_index) FROM book_chunks WHERE book_id = ?",
            )
            .bind(parsed_book_id)
            .fetch_one(self.pool)
            .await?;

            if indexed_chapter_count == 0 {
                return Ok(RagQueryResult {
                    answer: "Ainda não encontrei trechos indexados deste livro. Reindexe o livro para habilitar respostas contextualizadas.".to_string(),
                    source_level: "level1".to_string(),
                    source_label: "Resposta baseada em trechos específicos do livro".to_string(),
                });
            }

            let answer = build_missing_chapter_answer(
                chapter_number,
                indexed_chapter_count as usize,
                max_chapter_index.map(|index| (index as usize) + 1),
            );

            return Ok(RagQueryResult {
                answer,
                source_level: "level1".to_string(),
                source_label: "Diagnóstico de indexação por capítulo".to_string(),
            });
        }

        let rows = sqlx::query_as::<_, (String, i64, i64, i64, String)>(
            "SELECT be.embedding_json, be.chunk_id, bc.chapter_index, bc.char_start, bc.chunk_text
             FROM book_embeddings be
             JOIN book_chunks bc ON bc.id = be.chunk_id
             WHERE be.book_id = ?",
        )
        .bind(parsed_book_id)
        .fetch_all(self.pool)
        .await?;

        let mut scored: Vec<RankedChunk> = rows
            .into_iter()
            .filter_map(
                |(embedding_json, chunk_id, chapter_index, char_start, chunk_text)| {
                    let candidate: Vec<f32> = serde_json::from_str(&embedding_json).ok()?;
                    let score = cosine_similarity(&query_embedding, &candidate);
                    Some(RankedChunk {
                        chunk_id,
                        chapter_index,
                        char_start,
                        chunk_text,
                        similarity: score,
                        rerank_score: score,
                    })
                },
            )
            .collect();

        if scored.is_empty() {
            return Ok(RagQueryResult {
                answer: "Ainda não encontrei trechos indexados deste livro. Reindexe o livro para habilitar respostas contextualizadas.".to_string(),
                source_level: "level1".to_string(),
                source_label: "Resposta baseada em trechos específicos do livro".to_string(),
            });
        }

        scored.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut top_twenty: Vec<RankedChunk> = scored.into_iter().take(20).collect();
        let max_chapter = top_twenty
            .iter()
            .map(|chunk| chunk.chapter_index)
            .max()
            .unwrap_or(0);

        for chunk in &mut top_twenty {
            chunk.rerank_score = chunk.similarity
                + position_boost(&question_lower, chunk.chapter_index, max_chapter);
        }

        top_twenty.sort_by(|a, b| {
            b.rerank_score
                .partial_cmp(&a.rerank_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut selected: Vec<RankedChunk> = top_twenty.into_iter().take(5).collect();
        selected.sort_by_key(|chunk| (chunk.chapter_index, chunk.char_start, chunk.chunk_id));

        let mut context_chunks = Vec::new();
        for chunk in selected {
            let chapter_label = format!("[Capítulo {}]", chunk.chapter_index + 1);
            let short_text = trim_to_chars(&chunk.chunk_text, 800);
            context_chunks.push(format!("{} {}", chapter_label, short_text));
        }

        let context_text = if context_chunks.is_empty() {
            "Sem chunks indexados para este livro ainda.".to_string()
        } else {
            context_chunks.join("\n\n")
        };

        let prompt = format!(
            "Você é um assistente literário para leitura de romances e narrativas.\n\nContexto do livro:\n{}\n\nPergunta do usuário:\n{}\n\nResponda em português de forma objetiva, sem inventar fatos fora do contexto.",
            context_text, question,
        );

        let answer = self.generate_with_llm(prompt, 512).await?;
        Ok(RagQueryResult {
            answer,
            source_level: "level1".to_string(),
            source_label: "Resposta baseada em trechos específicos do livro".to_string(),
        })
    }

    async fn generate_from_summary(
        &self,
        question: &str,
        summary_text: &str,
        scope: &str,
    ) -> Result<String> {
        let prompt = format!(
            "Resumo contextual ({scope}):\n{}\n\nPergunta do usuário:\n{}\n\nResponda em português de forma objetiva sem inventar fatos fora do resumo.",
            trim_to_chars(summary_text, 3500),
            question,
        );

        self.generate_with_llm(prompt, 420).await
    }

    async fn generate_from_chapter_chunks(
        &self,
        question: &str,
        chapter_number: usize,
        chapter_chunks: &[String],
    ) -> Result<String> {
        let context_text = chapter_chunks
            .iter()
            .map(|chunk| trim_to_chars(chunk, 800))
            .collect::<Vec<String>>()
            .join("\n\n");

        let prompt = format!(
            "Trechos extraídos do capítulo {chapter_number} (ordem original):\n{context_text}\n\nPergunta do usuário:\n{question}\n\nResponda em português, de forma objetiva, sem inventar fatos fora dos trechos fornecidos.",
        );

        self.generate_with_llm(prompt, 420).await
    }

    async fn generate_with_llm(&self, prompt: String, max_tokens: i32) -> Result<String> {
        let llm = self.llm.clone();
        let output = task::spawn_blocking(move || llm.generate(&prompt, max_tokens))
            .await
            .map_err(|err| anyhow::anyhow!("Falha da tarefa de geração: {err}"))??;

        Ok(output)
    }
}

fn is_book_overview_question(question_lower: &str) -> bool {
    contains_any(
        question_lower,
        &[
            "resuma o livro",
            "resumo do livro",
            "visão geral",
            "visao geral",
            "sinopse",
            "sobre o livro como um todo",
        ],
    )
}

fn extract_requested_chapter(question: &str) -> Option<usize> {
    let normalized = question.to_ascii_lowercase();
    for marker in [
        "capítulo",
        "capitulo",
        "cap.",
        "cap ",
        "chapter",
        "ch.",
        "ch ",
    ] {
        let Some(idx) = normalized.find(marker) else {
            continue;
        };

        let tail = &normalized[idx + marker.len()..];
        let digits: String = tail
            .chars()
            .skip_while(|ch| !ch.is_ascii_digit())
            .take_while(|ch| ch.is_ascii_digit())
            .collect();

        if let Ok(number) = digits.parse::<usize>() {
            if number > 0 {
                return Some(number);
            }
        }
    }

    None
}

fn build_missing_chapter_answer(
    requested_chapter: usize,
    indexed_chapter_count: usize,
    max_chapter_number: Option<usize>,
) -> String {
    match max_chapter_number {
        Some(max_chapter) if requested_chapter > max_chapter => format!(
            "Não encontrei o Capítulo {requested_chapter} no índice deste livro. O maior capítulo indexado é o Capítulo {max_chapter}. Reindexe o livro ou confirme a numeração no EPUB.",
        ),
        Some(max_chapter) => format!(
            "Não encontrei conteúdo textual indexado para o Capítulo {requested_chapter}, embora existam capítulos indexados até o Capítulo {max_chapter}. Esse capítulo pode estar vazio, corrompido ou ter sido pulado na indexação.",
        ),
        None => format!(
            "Não encontrei capítulos indexados para este livro. Reindexe o livro e tente novamente (capítulos detectados: {indexed_chapter_count}).",
        ),
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn trim_to_chars(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    text.chars().take(max_chars).collect::<String>()
}

fn position_boost(question_lower: &str, chapter_index: i64, max_chapter_index: i64) -> f32 {
    if max_chapter_index <= 0 {
        return 0.0;
    }

    let position = chapter_index as f32 / max_chapter_index as f32;
    let mut boost = 0.0f32;

    if contains_any(
        question_lower,
        &["climax", "clímax", "ponto alto", "virada"],
    ) {
        let center = 0.65f32;
        let shape = (1.0 - ((position - center).abs() * 2.0)).max(0.0);
        boost += shape * 0.08;
    }

    if contains_any(
        question_lower,
        &["início", "inicio", "começo", "comeco", "abertura"],
    ) {
        boost += (1.0 - position) * 0.06;
    }

    if contains_any(
        question_lower,
        &["fim", "final", "desfecho", "conclusão", "conclusao"],
    ) {
        boost += position * 0.06;
    }

    boost
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let len = a.len().min(b.len());
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..len {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a.sqrt() * norm_b.sqrt())
}

#[cfg(test)]
mod tests {
    use super::{build_missing_chapter_answer, extract_requested_chapter};

    #[test]
    fn extracts_chapter_number_in_portuguese() {
        let chapter = extract_requested_chapter("Resuma o Capítulo 186");
        assert_eq!(chapter, Some(186));
    }

    #[test]
    fn extracts_chapter_number_in_english() {
        let chapter = extract_requested_chapter("Summarize chapter 42");
        assert_eq!(chapter, Some(42));
    }

    #[test]
    fn reports_out_of_range_chapter_with_max_available() {
        let answer = build_missing_chapter_answer(186, 120, Some(120));
        assert!(answer.contains("Capítulo 186"));
        assert!(answer.contains("Capítulo 120"));
    }

    #[test]
    fn reports_missing_chapter_inside_index_range() {
        let answer = build_missing_chapter_answer(12, 80, Some(80));
        assert!(answer.contains("Capítulo 12"));
        assert!(answer.contains("capítulos indexados até o Capítulo 80"));
    }
}
