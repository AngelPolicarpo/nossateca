use anyhow::Context;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use rbook::epub::Epub;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

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

#[derive(Debug, Clone)]
struct ManifestResource {
    canonical_path: String,
    media_type: String,
}

struct ResourceRewriter<'ebook> {
    epub: &'ebook Epub,
    resources: HashMap<String, ManifestResource>,
    data_url_cache: HashMap<String, String>,
}

impl<'ebook> ResourceRewriter<'ebook> {
    fn new(epub: &'ebook Epub) -> Self {
        let mut resources = HashMap::new();

        for entry in epub.manifest().iter() {
            let href = entry.href().path();
            let canonical_path = href.as_str().to_string();
            let decoded_path = href.decode().into_owned();

            let resource = ManifestResource {
                canonical_path: canonical_path.clone(),
                media_type: entry.media_type().to_string(),
            };

            resources
                .entry(canonical_path)
                .or_insert_with(|| resource.clone());

            if decoded_path != resource.canonical_path {
                resources.entry(decoded_path).or_insert(resource.clone());
            }
        }

        Self {
            epub,
            resources,
            data_url_cache: HashMap::new(),
        }
    }

    fn rewrite_html(&mut self, chapter_path: &str, html: &str) -> String {
        let tag_regex = Regex::new(r"(?is)<(?P<tag>[a-zA-Z][a-zA-Z0-9:-]*)(?P<attrs>[^<>]*)>")
            .expect("invalid tag regex");

        let mut rewritten = String::with_capacity(html.len());
        let mut cursor = 0;

        for capture in tag_regex.captures_iter(html) {
            let Some(full_match) = capture.get(0) else {
                continue;
            };

            rewritten.push_str(&html[cursor..full_match.start()]);

            let Some(tag_name_raw) = capture.name("tag").map(|item| item.as_str()) else {
                rewritten.push_str(full_match.as_str());
                cursor = full_match.end();
                continue;
            };

            let tag_name = tag_name_raw.to_ascii_lowercase();
            let attrs = capture
                .name("attrs")
                .map(|item| item.as_str())
                .unwrap_or_default();

            if !is_rewritable_tag(tag_name.as_str()) {
                rewritten.push_str(full_match.as_str());
                cursor = full_match.end();
                continue;
            }

            let rewritten_attrs = self.rewrite_tag_attributes(tag_name.as_str(), chapter_path, attrs);
            rewritten.push('<');
            rewritten.push_str(tag_name_raw);
            rewritten.push_str(rewritten_attrs.as_str());
            rewritten.push('>');

            cursor = full_match.end();
        }

        rewritten.push_str(&html[cursor..]);
        rewritten
    }

    fn rewrite_tag_attributes(&mut self, tag_name: &str, base_path: &str, attrs: &str) -> String {
        let attr_regex = Regex::new(
            r#"(?is)(?P<name>[a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*(?P<value>\"[^\"]*\"|'[^']*')"#,
        )
        .expect("invalid attribute regex");

        let rewrite_link_href = tag_name == "link" && is_stylesheet_link_tag(attrs);
        let mut output = String::with_capacity(attrs.len());
        let mut cursor = 0;

        for capture in attr_regex.captures_iter(attrs) {
            let Some(full_match) = capture.get(0) else {
                continue;
            };

            output.push_str(&attrs[cursor..full_match.start()]);

            let attr_name_raw = capture
                .name("name")
                .map(|item| item.as_str())
                .unwrap_or_default();
            let attr_name = attr_name_raw.to_ascii_lowercase();

            let raw_value = capture
                .name("value")
                .map(|item| item.as_str())
                .unwrap_or_default();
            let (quote_char, unquoted_value) = split_wrapped_quotes(raw_value);

            let rewritten_value = if attr_name == "srcset" && supports_srcset(tag_name) {
                Some(self.rewrite_srcset(base_path, unquoted_value))
            } else if should_rewrite_attribute(tag_name, attr_name.as_str(), rewrite_link_href) {
                self.resolve_to_data_url(base_path, unquoted_value, true)
            } else {
                None
            };

            if let Some(new_value) = rewritten_value {
                output.push_str(attr_name_raw);
                output.push('=');
                output.push(quote_char);
                output.push_str(new_value.as_str());
                output.push(quote_char);
            } else {
                output.push_str(full_match.as_str());
            }

            cursor = full_match.end();
        }

        output.push_str(&attrs[cursor..]);
        output
    }

    fn rewrite_srcset(&mut self, base_path: &str, srcset: &str) -> String {
        srcset
            .split(',')
            .map(|candidate| {
                let trimmed = candidate.trim();
                if trimmed.is_empty() {
                    return String::new();
                }

                let mut tokens = trimmed.split_whitespace();
                let Some(raw_url_token) = tokens.next() else {
                    return trimmed.to_string();
                };

                let descriptor = tokens.collect::<Vec<_>>().join(" ");
                let (quote_char, raw_url) = split_optional_quotes(raw_url_token);

                let rewritten_url = self
                    .resolve_to_data_url(base_path, raw_url, true)
                    .unwrap_or_else(|| raw_url.to_string());

                let resolved = if let Some(quote) = quote_char {
                    format!("{quote}{rewritten_url}{quote}")
                } else {
                    rewritten_url
                };

                if descriptor.is_empty() {
                    resolved
                } else {
                    format!("{resolved} {descriptor}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn rewrite_css_urls(&mut self, css_path: &str, css: &str) -> String {
        let url_regex =
            Regex::new(r#"(?is)url\(\s*(?P<value>\"[^\"]*\"|'[^']*'|[^)]+?)\s*\)"#)
                .expect("invalid css url regex");

        url_regex
            .replace_all(css, |capture: &regex::Captures<'_>| {
                let original = capture
                    .get(0)
                    .map(|item| item.as_str())
                    .unwrap_or_default();

                let wrapped_value = capture
                    .name("value")
                    .map(|item| item.as_str())
                    .unwrap_or_default();

                let (quote_char, raw_value) = split_optional_quotes(wrapped_value.trim());

                let Some(rewritten) = self.resolve_to_data_url(css_path, raw_value, false) else {
                    return original.to_string();
                };

                if let Some(quote) = quote_char {
                    format!("url({quote}{rewritten}{quote})")
                } else {
                    format!("url({rewritten})")
                }
            })
            .into_owned()
    }

    fn resolve_to_data_url(
        &mut self,
        base_path: &str,
        raw_reference: &str,
        rewrite_nested_css_urls: bool,
    ) -> Option<String> {
        let trimmed_reference = raw_reference.trim();
        if should_skip_resource_reference(trimmed_reference) {
            return None;
        }

        let fragment = extract_fragment(trimmed_reference);
        let reference_path = strip_query_and_fragment(trimmed_reference);
        if reference_path.is_empty() {
            return None;
        }

        let resolved_path = resolve_virtual_path(base_path, reference_path)?;
        let resource = self.lookup_resource(&resolved_path);

        let canonical_path = resource
            .as_ref()
            .map(|item| item.canonical_path.clone())
            .unwrap_or_else(|| resolved_path.clone());

        let cache_key = format!(
            "{}|inline_css:{}",
            canonical_path,
            if rewrite_nested_css_urls { "true" } else { "false" }
        );

        if let Some(cached) = self.data_url_cache.get(&cache_key) {
            return Some(append_fragment(cached, fragment));
        }

        let mut bytes = self.read_resource_bytes(canonical_path.as_str())?;
        let mut media_type = resource
            .as_ref()
            .map(|item| item.media_type.clone())
            .unwrap_or_else(|| guess_mime_from_path(canonical_path.as_str()).to_string());

        if media_type.trim().is_empty() {
            media_type = guess_mime_from_path(canonical_path.as_str()).to_string();
        }

        if rewrite_nested_css_urls && media_type.to_ascii_lowercase().starts_with("text/css") {
            let css_text = String::from_utf8_lossy(&bytes).to_string();
            bytes = self
                .rewrite_css_urls(canonical_path.as_str(), css_text.as_str())
                .into_bytes();
        }

        let data_url = format!(
            "data:{};base64,{}",
            media_type,
            BASE64_STANDARD.encode(bytes)
        );

        self.data_url_cache.insert(cache_key, data_url.clone());
        Some(append_fragment(data_url.as_str(), fragment))
    }

    fn lookup_resource(&self, resolved_path: &str) -> Option<ManifestResource> {
        if let Some(resource) = self.resources.get(resolved_path) {
            return Some(resource.clone());
        }

        if resolved_path.contains(' ') {
            let encoded = resolved_path.replace(' ', "%20");
            if let Some(resource) = self.resources.get(encoded.as_str()) {
                return Some(resource.clone());
            }
        }

        if resolved_path.contains("%20") {
            let decoded = resolved_path.replace("%20", " ");
            if let Some(resource) = self.resources.get(decoded.as_str()) {
                return Some(resource.clone());
            }
        }

        None
    }

    fn read_resource_bytes(&self, canonical_path: &str) -> Option<Vec<u8>> {
        if let Ok(bytes) = self.epub.read_resource_bytes(canonical_path) {
            return Some(bytes);
        }

        if canonical_path.contains(' ') {
            let encoded = canonical_path.replace(' ', "%20");
            if let Ok(bytes) = self.epub.read_resource_bytes(encoded.as_str()) {
                return Some(bytes);
            }
        }

        if canonical_path.contains("%20") {
            let decoded = canonical_path.replace("%20", " ");
            if let Ok(bytes) = self.epub.read_resource_bytes(decoded.as_str()) {
                return Some(bytes);
            }
        }

        None
    }
}

impl EpubParser {
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
        }
    }

    pub fn extract_metadata(file_path: &str) -> Result<EpubMetadata, anyhow::Error> {
        let epub =
            Epub::open(file_path).with_context(|| format!("failed to open EPUB: {}", file_path))?;

        let metadata = epub.metadata();

        let fallback_title = fallback_title_from_path(file_path);
        let title = metadata
            .title()
            .and_then(|item| non_empty(item.value()))
            .unwrap_or(fallback_title);

        let author = metadata.creators().find_map(|item| non_empty(item.value()));

        let language = metadata
            .language()
            .and_then(|item| non_empty(item.value()));

        let isbn = metadata
            .identifiers()
            .find_map(|identifier| {
                let value = non_empty(identifier.value())?;

                let scheme_is_isbn = identifier
                    .scheme()
                    .map(|scheme| scheme.code().to_ascii_lowercase().contains("isbn"))
                    .unwrap_or(false);

                if scheme_is_isbn || looks_like_isbn(value.as_str()) {
                    Some(value)
                } else {
                    None
                }
            })
            .or_else(|| metadata.identifier().and_then(|identifier| non_empty(identifier.value())));

        Ok(EpubMetadata {
            title,
            author,
            language,
            isbn,
        })
    }

    pub fn get_spine(&self) -> Vec<String> {
        match Epub::open(&self.file_path) {
            Ok(epub) => {
                let mut readable = Vec::new();

                for spine_entry in epub.spine().iter() {
                    let Some(manifest_entry) = spine_entry.manifest_entry() else {
                        continue;
                    };

                    if !is_probably_text_mime(manifest_entry.media_type()) {
                        continue;
                    }

                    if let Ok(content) = manifest_entry.read_str() {
                        if !content.trim().is_empty() {
                            readable.push(spine_entry.idref().to_string());
                        }
                    }
                }

                readable
            }
            Err(_) => Vec::new(),
        }
    }

    pub fn get_chapter_content(&self, chapter_id: &str) -> Result<String, anyhow::Error> {
        let epub = Epub::open(&self.file_path)
            .with_context(|| format!("failed to open EPUB: {}", self.file_path))?;

        let spine_entry = epub
            .spine()
            .by_idref(chapter_id)
            .next()
            .ok_or_else(|| anyhow::anyhow!("chapter not found: {}", chapter_id))?;

        let chapter = spine_entry
            .manifest_entry()
            .ok_or_else(|| anyhow::anyhow!("failed to resolve chapter manifest entry"))?;

        let chapter_path = chapter.href().path().as_str().to_string();

        let html = chapter
            .read_str()
            .with_context(|| format!("failed to read chapter content for '{}': {}", chapter_id, chapter_path))?;

        let mut rewriter = ResourceRewriter::new(&epub);

        Ok(rewriter.rewrite_html(chapter_path.as_str(), html.as_str()))
    }

    pub fn resolve_internal_link(
        &self,
        current_chapter_index: usize,
        href: &str,
    ) -> Result<Option<(usize, Option<String>)>, anyhow::Error> {
        let trimmed_href = href.trim();
        if trimmed_href.is_empty() || should_skip_navigation_reference(trimmed_href) {
            return Ok(None);
        }

        let epub = Epub::open(&self.file_path)
            .with_context(|| format!("failed to open EPUB: {}", self.file_path))?;

        let mut spine_paths: Vec<String> = Vec::new();
        for spine_entry in epub.spine().iter() {
            let Some(manifest_entry) = spine_entry.manifest_entry() else {
                continue;
            };

            if !is_probably_text_mime(manifest_entry.media_type()) {
                continue;
            }

            spine_paths.push(manifest_entry.href().path().as_str().to_string());
        }

        if spine_paths.is_empty() {
            return Ok(None);
        }

        let safe_current_index = current_chapter_index.min(spine_paths.len().saturating_sub(1));
        let anchor_id = extract_fragment(trimmed_href)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        let reference_path = strip_query_and_fragment(trimmed_href);
        if reference_path.is_empty() {
            return Ok(Some((safe_current_index, anchor_id)));
        }

        let current_path = spine_paths
            .get(safe_current_index)
            .map(String::as_str)
            .ok_or_else(|| anyhow::anyhow!("chapter index out of bounds"))?;

        let Some(resolved_path) = resolve_virtual_path(current_path, reference_path) else {
            return Ok(None);
        };

        let mut path_to_index: HashMap<String, usize> = HashMap::new();
        for (index, path) in spine_paths.iter().enumerate() {
            insert_navigation_path_variants(&mut path_to_index, path.as_str(), index);
        }

        let normalized_resolved =
            normalize_navigation_path(resolved_path.as_str()).unwrap_or(resolved_path);

        let target_index = path_to_index.get(normalized_resolved.as_str()).copied();
        Ok(target_index.map(|index| (index, anchor_id)))
    }

    pub fn get_toc(&self) -> Vec<(String, String)> {
        match Epub::open(&self.file_path) {
            Ok(epub) => {
                let mut toc = Vec::new();
                let mut seen = HashSet::new();

                if let Some(contents_root) = epub.toc().contents() {
                    for entry in contents_root.flatten() {
                        let title = entry.label().trim();
                        if title.is_empty() {
                            continue;
                        }

                        let Some(manifest_entry) = entry.manifest_entry() else {
                            continue;
                        };

                        let chapter_id = manifest_entry.id().to_string();
                        if seen.insert(chapter_id.clone()) {
                            toc.push((title.to_string(), chapter_id));
                        }
                    }
                }

                if toc.is_empty() {
                    for (index, chapter_id) in self.get_spine().into_iter().enumerate() {
                        toc.push((format!("Capitulo {}", index + 1), chapter_id));
                    }
                }

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

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn looks_like_isbn(value: &str) -> bool {
    let normalized = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();

    if normalized.len() == 13 {
        return normalized.chars().all(|ch| ch.is_ascii_digit());
    }

    if normalized.len() != 10 {
        return false;
    }

    let mut chars = normalized.chars();
    let first_nine_are_digits = chars.by_ref().take(9).all(|ch| ch.is_ascii_digit());
    let last_is_digit_or_x = chars
        .next()
        .map(|ch| ch.is_ascii_digit() || ch == 'X' || ch == 'x')
        .unwrap_or(false);

    first_nine_are_digits && last_is_digit_or_x
}

fn is_rewritable_tag(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "img" | "image" | "source" | "video" | "audio" | "track" | "object" | "link"
    )
}

fn supports_srcset(tag_name: &str) -> bool {
    matches!(tag_name, "img" | "source")
}

fn should_rewrite_attribute(tag_name: &str, attr_name: &str, rewrite_link_href: bool) -> bool {
    match attr_name {
        "src" => matches!(tag_name, "img" | "audio" | "video" | "source" | "track"),
        "poster" => tag_name == "video",
        "data" => tag_name == "object",
        "href" => (tag_name == "image") || (tag_name == "link" && rewrite_link_href),
        "xlink:href" => tag_name == "image",
        _ => false,
    }
}

fn is_stylesheet_link_tag(attrs: &str) -> bool {
    let lower = attrs.to_ascii_lowercase();
    lower.contains("rel=\"stylesheet\"")
        || lower.contains("rel='stylesheet'")
        || lower.contains("rel=stylesheet")
}

fn split_wrapped_quotes(value: &str) -> (char, &str) {
    let bytes = value.as_bytes();
    if bytes.len() < 2 {
        return ('\"', value);
    }

    let first = bytes[0] as char;
    let last = bytes[bytes.len() - 1] as char;

    if (first == '\"' && last == '\"') || (first == '\'' && last == '\'') {
        (first, &value[1..value.len() - 1])
    } else {
        ('\"', value)
    }
}

fn split_optional_quotes(value: &str) -> (Option<char>, &str) {
    let trimmed = value.trim();
    if trimmed.len() < 2 {
        return (None, trimmed);
    }

    let bytes = trimmed.as_bytes();
    let first = bytes[0] as char;
    let last = bytes[bytes.len() - 1] as char;

    if (first == '\"' && last == '\"') || (first == '\'' && last == '\'') {
        (Some(first), &trimmed[1..trimmed.len() - 1])
    } else {
        (None, trimmed)
    }
}

fn should_skip_resource_reference(reference: &str) -> bool {
    let lower = reference.trim().to_ascii_lowercase();

    lower.is_empty()
        || lower.starts_with('#')
        || lower.starts_with("data:")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("javascript:")
        || lower.starts_with("blob:")
        || lower.starts_with("//")
}

fn extract_fragment(reference: &str) -> Option<&str> {
    reference
        .find('#')
        .and_then(|index| reference.get(index + 1..))
        .filter(|fragment| !fragment.is_empty())
}

fn append_fragment(base: &str, fragment: Option<&str>) -> String {
    if let Some(value) = fragment {
        format!("{}#{}", base, value)
    } else {
        base.to_string()
    }
}

fn strip_query_and_fragment(reference: &str) -> &str {
    let query_index = reference.find('?');
    let fragment_index = reference.find('#');

    let cutoff = match (query_index, fragment_index) {
        (Some(query), Some(fragment)) => query.min(fragment),
        (Some(query), None) => query,
        (None, Some(fragment)) => fragment,
        (None, None) => reference.len(),
    };

    &reference[..cutoff]
}

fn should_skip_navigation_reference(reference: &str) -> bool {
    let lower = reference.trim().to_ascii_lowercase();

    lower.is_empty()
        || lower.starts_with("data:")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("mailto:")
        || lower.starts_with("tel:")
        || lower.starts_with("javascript:")
        || lower.starts_with("blob:")
        || lower.starts_with("file:")
        || lower.starts_with("//")
}

fn resolve_virtual_path(base_path: &str, reference_path: &str) -> Option<String> {
    let base_dir = Path::new(base_path).parent().unwrap_or_else(|| Path::new("/"));

    let joined = if reference_path.starts_with('/') {
        PathBuf::from(reference_path)
    } else {
        base_dir.join(reference_path)
    };

    normalize_virtual_path(joined)
}

fn normalize_navigation_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized_input = if trimmed.starts_with('/') {
        PathBuf::from(trimmed)
    } else {
        PathBuf::from(format!("/{}", trimmed))
    };

    normalize_virtual_path(normalized_input)
}

fn insert_navigation_path_variants(
    path_to_index: &mut HashMap<String, usize>,
    path: &str,
    index: usize,
) {
    if let Some(normalized) = normalize_navigation_path(path) {
        path_to_index.entry(normalized).or_insert(index);
    }

    if path.contains("%20") {
        let decoded_spaces = path.replace("%20", " ");
        if let Some(normalized) = normalize_navigation_path(decoded_spaces.as_str()) {
            path_to_index.entry(normalized).or_insert(index);
        }
    }

    if path.contains(' ') {
        let encoded_spaces = path.replace(' ', "%20");
        if let Some(normalized) = normalize_navigation_path(encoded_spaces.as_str()) {
            path_to_index.entry(normalized).or_insert(index);
        }
    }
}

fn normalize_virtual_path(path: PathBuf) -> Option<String> {
    let mut segments = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(_) => {}
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                segments.pop();
            }
            Component::Normal(value) => segments.push(value.to_string_lossy().to_string()),
        }
    }

    if segments.is_empty() {
        return None;
    }

    Some(format!("/{}", segments.join("/")))
}

fn guess_mime_from_path(path: &str) -> &'static str {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("css") => "text/css",
        Some("js") => "application/javascript",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("otf") => "font/otf",
        Some("xhtml") => "application/xhtml+xml",
        Some("html") | Some("htm") => "text/html",
        Some("mp3") => "audio/mpeg",
        Some("m4a") => "audio/mp4",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_virtual_path, should_skip_resource_reference, strip_query_and_fragment};
    use std::path::PathBuf;

    #[test]
    fn normalizes_virtual_path_with_parent_segments() {
        let path = PathBuf::from("/EPUB/chapters/../images/cover.png");
        let normalized = normalize_virtual_path(path);

        assert_eq!(normalized.as_deref(), Some("/EPUB/images/cover.png"));
    }

    #[test]
    fn strips_query_and_fragment_from_reference() {
        assert_eq!(strip_query_and_fragment("images/cover.png?v=1#id"), "images/cover.png");
        assert_eq!(strip_query_and_fragment("chapter.xhtml#intro"), "chapter.xhtml");
    }

    #[test]
    fn skips_external_or_data_resource_references() {
        assert!(should_skip_resource_reference("https://example.com/image.png"));
        assert!(should_skip_resource_reference("data:image/png;base64,AAAA"));
        assert!(should_skip_resource_reference("#chapter-1"));
        assert!(!should_skip_resource_reference("../images/figure-1.png"));
    }
}
