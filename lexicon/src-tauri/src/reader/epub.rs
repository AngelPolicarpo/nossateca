use anyhow::Context;
use std::io::{Read, Seek};
use std::path::Path;

pub struct EpubParser {
    file_path: String,
}

#[derive(Debug, Clone)]
pub struct EpubMetadata {
    pub title: String,
    pub author: Option<String>,
    pub language: Option<String>,
    pub isbn: Option<String>,
}

impl EpubParser {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
        }
    }

    pub fn extract_metadata(file_path: &str) -> Result<EpubMetadata, anyhow::Error> {
        let doc = epub::doc::EpubDoc::new(file_path)
            .with_context(|| format!("failed to open EPUB: {}", file_path))?;

        let fallback_title = fallback_title_from_path(file_path);
        let title = doc
            .mdata("title")
            .map(|item| item.value.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(fallback_title);

        let author = doc
            .mdata("creator")
            .map(|item| item.value.clone())
            .or_else(|| doc.mdata("author").map(|item| item.value.clone()))
            .filter(|value| !value.trim().is_empty());

        let language = doc
            .mdata("language")
            .map(|item| item.value.clone())
            .filter(|value| !value.trim().is_empty());

        let isbn = doc
            .mdata("identifier")
            .map(|item| item.value.clone())
            .filter(|value| !value.trim().is_empty());

        Ok(EpubMetadata {
            title,
            author,
            language,
            isbn,
        })
    }

    pub fn get_spine(&self) -> Vec<String> {
        match epub::doc::EpubDoc::new(&self.file_path) {
            Ok(mut doc) => {
                let mut readable = Vec::new();

                for index in 0..doc.spine.len() {
                    let chapter_id = doc.spine[index].idref.clone();

                    if !doc.set_current_chapter(index) {
                        continue;
                    }

                    let has_content = read_current_text(&mut doc)
                        .map(|content| !content.trim().is_empty())
                        .unwrap_or(false);

                    if has_content {
                        readable.push(chapter_id);
                    }
                }

                readable
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn get_chapter_content(&self, chapter_id: &str) -> Result<String, anyhow::Error> {
        let mut doc = epub::doc::EpubDoc::new(&self.file_path)
            .with_context(|| format!("failed to open EPUB: {}", self.file_path))?;

        let chapter_index = doc
            .spine
            .iter()
            .position(|item| item.idref == chapter_id)
            .ok_or_else(|| anyhow::anyhow!("chapter not found: {}", chapter_id))?;

        if !doc.set_current_chapter(chapter_index) {
            return Err(anyhow::anyhow!("failed to set current chapter"));
        }

        let html = read_current_text(&mut doc)
            .ok_or_else(|| anyhow::anyhow!("failed to read chapter content"))?;

        Ok(sanitize_chapter_html(&html))
    }

    pub fn get_toc(&self) -> Vec<(String, String)> {
        fn flatten_nav_points(
            out: &mut Vec<(String, String)>,
            parser_doc: &epub::doc::EpubDoc<std::io::BufReader<std::fs::File>>,
            points: &[epub::doc::NavPoint],
        ) {
            for point in points {
                if let Some(chapter_index) = parser_doc.resource_uri_to_chapter(&point.content) {
                    if let Some(spine_item) = parser_doc.spine.get(chapter_index) {
                        out.push((point.label.clone(), spine_item.idref.clone()));
                    }
                }

                flatten_nav_points(out, parser_doc, &point.children);
            }
        }

        match epub::doc::EpubDoc::new(&self.file_path) {
            Ok(doc) => {
                let mut toc = Vec::new();
                flatten_nav_points(&mut toc, &doc, &doc.toc);
                toc
            }
            Err(_) => Vec::new(),
        }
    }
}

fn fallback_title_from_path(file_path: &str) -> String {
    Path::new(file_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unknown Title".to_string())
}

fn is_probably_text_mime(mime: &str) -> bool {
    let lower = mime.to_ascii_lowercase();
    lower.starts_with("text/")
        || lower.contains("html")
        || lower.contains("xhtml")
        || lower.contains("xml")
}

fn read_current_text<R: Read + Seek>(doc: &mut epub::doc::EpubDoc<R>) -> Option<String> {
    if let Some((content, _mime)) = doc.get_current_str() {
        return Some(content);
    }

    let (bytes, mime) = doc.get_current()?;
    if !is_probably_text_mime(&mime) {
        return None;
    }

    Some(String::from_utf8_lossy(&bytes).to_string())
}

fn sanitize_chapter_html(html: &str) -> String {
    strip_stylesheet_link_tags(html)
}

fn strip_stylesheet_link_tags(html: &str) -> String {
    let lower_html = html.to_ascii_lowercase();
    let mut output = String::with_capacity(html.len());
    let mut cursor = 0;

    while let Some(found) = lower_html[cursor..].find("<link") {
        let tag_start = cursor + found;
        output.push_str(&html[cursor..tag_start]);

        let Some(end) = lower_html[tag_start..].find('>') else {
            output.push_str(&html[tag_start..]);
            cursor = html.len();
            break;
        };

        let tag_end = tag_start + end + 1;
        let lower_tag = &lower_html[tag_start..tag_end];
        if !is_stylesheet_link_tag(lower_tag) {
            output.push_str(&html[tag_start..tag_end]);
        }

        cursor = tag_end;
    }

    if cursor < html.len() {
        output.push_str(&html[cursor..]);
    }

    output
}

fn is_stylesheet_link_tag(lower_tag: &str) -> bool {
    lower_tag.contains("rel=\"stylesheet\"")
        || lower_tag.contains("rel='stylesheet'")
        || lower_tag.contains("rel=stylesheet")
        || lower_tag.contains("type=\"text/css\"")
        || lower_tag.contains("type='text/css'")
        || lower_tag.contains(".css")
}

#[cfg(test)]
mod tests {
    use super::sanitize_chapter_html;

    #[test]
    fn strips_stylesheet_link_tags() {
        let input = r#"<html><head><link rel=\"stylesheet\" href=\"style/style.css\" /><link rel=\"icon\" href=\"book.ico\" /></head><body><p>oi</p></body></html>"#;
        let output = sanitize_chapter_html(input);

        assert!(!output.contains("style/style.css"));
        assert!(output.contains("book.ico"));
        assert!(output.contains("<p>oi</p>"));
    }

    #[test]
    fn keeps_non_stylesheet_links() {
        let input = r#"<head><link rel=\"preconnect\" href=\"https://example.com\" /></head>"#;
        let output = sanitize_chapter_html(input);

        assert_eq!(output, input);
    }
}
