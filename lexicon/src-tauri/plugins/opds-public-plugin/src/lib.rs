wit_bindgen::generate!({
    path: "../../wit/search-plugin.wit",
    world: "search-plugin",
});

struct OpdsPublicPlugin;

export!(OpdsPublicPlugin);

struct CatalogEntry {
    id: &'static str,
    title: &'static str,
    author: &'static str,
    format: &'static str,
    download_url: &'static str,
}

const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        id: "opds-pride-and-prejudice",
        title: "Pride and Prejudice",
        author: "Jane Austen",
        format: "epub",
        download_url: "https://standardebooks.org/ebooks/jane-austen/pride-and-prejudice/downloads/jane-austen_pride-and-prejudice.epub",
    },
    CatalogEntry {
        id: "opds-frankenstein",
        title: "Frankenstein",
        author: "Mary Shelley",
        format: "epub",
        download_url: "https://standardebooks.org/ebooks/mary-shelley/frankenstein/downloads/mary-shelley_frankenstein.epub",
    },
    CatalogEntry {
        id: "opds-dorian-gray",
        title: "The Picture of Dorian Gray",
        author: "Oscar Wilde",
        format: "epub",
        download_url: "https://standardebooks.org/ebooks/oscar-wilde/the-picture-of-dorian-gray/downloads/oscar-wilde_the-picture-of-dorian-gray.epub",
    },
    CatalogEntry {
        id: "opds-metamorphosis",
        title: "The Metamorphosis",
        author: "Franz Kafka",
        format: "epub",
        download_url: "https://standardebooks.org/ebooks/franz-kafka/the-metamorphosis/downloads/franz-kafka_the-metamorphosis.epub",
    },
];

impl Guest for OpdsPublicPlugin {
    fn search_books(request: SearchRequest) -> Vec<SearchResult> {
        let normalized_query = request.query.trim().to_ascii_lowercase();
        if normalized_query.is_empty() {
            return Vec::new();
        }

        let terms: Vec<&str> = normalized_query.split_whitespace().collect();
        if terms.is_empty() {
            return Vec::new();
        }

        let mut matches = Vec::new();

        for item in CATALOG {
            let haystack = format!("{} {}", item.title, item.author).to_ascii_lowercase();
            let matched_terms = terms.iter().filter(|term| haystack.contains(**term)).count();
            if matched_terms == 0 {
                continue;
            }

            let match_ratio = matched_terms as f32 / terms.len() as f32;
            let score = (0.60 + match_ratio * 0.35).min(0.98);

            matches.push(SearchResult {
                id: item.id.to_string(),
                title: item.title.to_string(),
                author: Some(item.author.to_string()),
                source: "opds-public".to_string(),
                format: Some(item.format.to_string()),
                download_url: item.download_url.to_string(),
                score,
            });
        }

        if matches.is_empty() {
            matches.push(SearchResult {
                id: format!("opds-fallback-{}", slugify(&normalized_query)),
                title: format!("OPDS Catalog Search: {}", request.query.trim()),
                author: None,
                source: "opds-public".to_string(),
                format: Some("atom".to_string()),
                download_url: format!(
                    "https://www.feedbooks.com/publicdomain/catalog.atom?query={}",
                    url_encode(request.query.trim())
                ),
                score: 0.58,
            });
        }

        matches
    }
}

fn slugify(value: &str) -> String {
    let mut output = String::new();

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            continue;
        }

        if character.is_whitespace() && !output.ends_with('-') {
            output.push('-');
        }
    }

    output.trim_matches('-').to_string()
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();

    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }

    encoded
}
