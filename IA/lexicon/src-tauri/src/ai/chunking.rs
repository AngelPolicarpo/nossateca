#[derive(Debug, Clone)]
pub struct Chunk {
    pub chunk_text: String,
    pub char_start: usize,
    pub char_end: usize,
}

pub const CHUNK_SIZE: usize = 700;
pub const CHUNK_OVERLAP: usize = 120;

#[derive(Debug, Clone)]
struct TextUnit {
    text: String,
    token_count: usize,
}

pub fn chunk_literary_text(text: &str) -> Vec<Chunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let units = paragraph_sentence_units(text);
    if units.is_empty() {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current: Vec<TextUnit> = Vec::new();
    let mut current_tokens = 0usize;
    let mut cursor = 0usize;

    for unit in units {
        if current_tokens + unit.token_count > CHUNK_SIZE && !current.is_empty() {
            flush_chunk(&mut chunks, &current, &mut cursor);

            let overlap_units = build_overlap_units(&current);
            current_tokens = overlap_units.iter().map(|item| item.token_count).sum();
            current = overlap_units;
        }

        current_tokens += unit.token_count;
        current.push(unit);
    }

    if !current.is_empty() {
        flush_chunk(&mut chunks, &current, &mut cursor);
    }

    chunks
}

fn paragraph_sentence_units(text: &str) -> Vec<TextUnit> {
    let paragraphs: Vec<&str> = text
        .split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .collect();

    let mut units = Vec::new();

    for paragraph in paragraphs {
        let token_count = count_tokens(paragraph);
        if token_count <= CHUNK_SIZE {
            units.push(TextUnit {
                text: paragraph.to_string(),
                token_count,
            });
            continue;
        }

        let sentences = split_sentences(paragraph);
        if sentences.is_empty() {
            units.extend(split_words_fallback(paragraph, CHUNK_SIZE));
            continue;
        }

        let mut sentence_group = Vec::new();
        let mut group_tokens = 0usize;

        for sentence in sentences {
            let sentence_tokens = count_tokens(&sentence);
            if sentence_tokens > CHUNK_SIZE {
                if !sentence_group.is_empty() {
                    units.push(TextUnit {
                        text: sentence_group.join(" "),
                        token_count: group_tokens,
                    });
                    sentence_group.clear();
                    group_tokens = 0;
                }

                units.extend(split_words_fallback(&sentence, CHUNK_SIZE));
                continue;
            }

            if group_tokens + sentence_tokens > CHUNK_SIZE && !sentence_group.is_empty() {
                units.push(TextUnit {
                    text: sentence_group.join(" "),
                    token_count: group_tokens,
                });
                sentence_group.clear();
                group_tokens = 0;
            }

            group_tokens += sentence_tokens;
            sentence_group.push(sentence);
        }

        if !sentence_group.is_empty() {
            units.push(TextUnit {
                text: sentence_group.join(" "),
                token_count: group_tokens,
            });
        }
    }

    units
}

fn split_sentences(paragraph: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0usize;
    let mut chars = paragraph.char_indices().peekable();

    while let Some((idx, ch)) = chars.next() {
        let is_sentence_end = matches!(ch, '.' | '!' | '?' | ';' | ':');
        if !is_sentence_end {
            continue;
        }

        let end = idx + ch.len_utf8();
        let next_is_boundary = chars
            .peek()
            .map(|(_, next)| next.is_whitespace())
            .unwrap_or(true);

        if !next_is_boundary {
            continue;
        }

        let sentence = paragraph[start..end].trim();
        if !sentence.is_empty() {
            sentences.push(sentence.to_string());
        }
        start = end;
    }

    if start < paragraph.len() {
        let sentence = paragraph[start..].trim();
        if !sentence.is_empty() {
            sentences.push(sentence.to_string());
        }
    }

    sentences
}

fn split_words_fallback(text: &str, max_tokens: usize) -> Vec<TextUnit> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Vec::new();
    }

    let mut units = Vec::new();
    let mut start = 0usize;

    while start < words.len() {
        let end = (start + max_tokens).min(words.len());
        let slice = words[start..end].join(" ");
        units.push(TextUnit {
            text: slice,
            token_count: end - start,
        });
        start = end;
    }

    units
}

fn build_overlap_units(units: &[TextUnit]) -> Vec<TextUnit> {
    let mut overlap = Vec::new();
    let mut tokens = 0usize;

    for unit in units.iter().rev() {
        if tokens >= CHUNK_OVERLAP {
            break;
        }

        overlap.push(unit.clone());
        tokens += unit.token_count;
    }

    overlap.reverse();
    overlap
}

fn flush_chunk(chunks: &mut Vec<Chunk>, units: &[TextUnit], cursor: &mut usize) {
    if units.is_empty() {
        return;
    }

    let chunk_text = units
        .iter()
        .map(|unit| unit.text.as_str())
        .collect::<Vec<&str>>()
        .join("\n\n");

    let char_start = *cursor;
    let char_end = char_start + chunk_text.len();
    chunks.push(Chunk {
        chunk_text,
        char_start,
        char_end,
    });

    let overlap_text_len = build_overlap_units(units)
        .iter()
        .map(|unit| unit.text.len())
        .sum::<usize>();

    *cursor = char_end.saturating_sub(overlap_text_len);
}

fn count_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}
