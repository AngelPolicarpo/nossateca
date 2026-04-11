wit_bindgen::generate!({
    path: "../../wit/search-plugin.wit",
    world: "search-plugin",
});

struct MockSearchPlugin;

export!(MockSearchPlugin);

impl Guest for MockSearchPlugin {
    fn search_books(request: SearchRequest) -> Vec<SearchResult> {
        let normalized_query = request.query.trim();
        if normalized_query.is_empty() {
            return Vec::new();
        }

        vec![
            SearchResult {
                id: format!("mock-{}-1", slugify(normalized_query)),
                title: format!("{} para leitura guiada", capitalize(normalized_query)),
                author: Some("Plugin Mock Lexicon".to_string()),
                source: "mock-search-plugin".to_string(),
                format: Some("epub".to_string()),
                download_url: format!(
                    "https://example.com/mock/{}-guided.epub",
                    slugify(normalized_query)
                ),
                score: 0.93,
            },
            SearchResult {
                id: format!("mock-{}-2", slugify(normalized_query)),
                title: format!("Manual essencial de {}", capitalize(normalized_query)),
                author: Some("Autor de Exemplo".to_string()),
                source: "mock-search-plugin".to_string(),
                format: Some("pdf".to_string()),
                download_url: format!(
                    "https://example.com/mock/{}-manual.pdf",
                    slugify(normalized_query)
                ),
                score: 0.87,
            },
        ]
    }
}

fn slugify(value: &str) -> String {
    let mut output = String::new();

    for char in value.chars() {
        if char.is_ascii_alphanumeric() {
            output.push(char.to_ascii_lowercase());
            continue;
        }

        if (char.is_whitespace() || char == '-' || char == '_') && !output.ends_with('-') {
            output.push('-');
        }
    }

    output.trim_matches('-').to_string()
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();

    match chars.next() {
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
        None => String::new(),
    }
}
