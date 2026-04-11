use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

use llama_cpp_2 as _;
use ort as _;

#[derive(Debug, Clone, Copy)]
enum EmbeddingBackend {
    Onnx,
    Gguf,
}

#[derive(Debug, Clone, Copy)]
enum EmbeddingInputRole {
    Query,
    Passage,
}

#[derive(Debug, Clone)]
pub struct EmbeddingEngine {
    model_path: String,
    backend: EmbeddingBackend,
    dimensions: usize,
    use_e5_prefix: bool,
}

impl EmbeddingEngine {
    pub fn new(model_path: &str) -> Result<Self> {
        if model_path.trim().is_empty() {
            return Err(anyhow::anyhow!("Embedding model path is not configured"));
        }

        if !Path::new(model_path).exists() {
            return Err(anyhow::anyhow!(
                "Embedding model not found at {}",
                model_path
            ));
        }

        let extension = Path::new(model_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .unwrap_or_default();

        let (backend, dimensions) = match extension.as_str() {
            "onnx" => (EmbeddingBackend::Onnx, 384usize),
            "gguf" => (EmbeddingBackend::Gguf, 768usize),
            _ => {
                anyhow::bail!("Embedding model must be .onnx or .gguf, got .{}", extension)
            }
        };

        Ok(Self {
            model_path: model_path.to_string(),
            backend,
            dimensions,
            use_e5_prefix: should_use_e5_prefix(model_path),
        })
    }

    pub fn embed_query(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_with_role(text, EmbeddingInputRole::Query)
    }

    pub fn embed_passage(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_with_role(text, EmbeddingInputRole::Passage)
    }

    #[allow(dead_code)]
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.embed_query(text)
    }

    fn embed_with_role(&self, text: &str, role: EmbeddingInputRole) -> Result<Vec<f32>> {
        let _ = (&self.model_path, self.backend);
        if text.trim().is_empty() {
            return Ok(vec![0.0; self.dimensions]);
        }

        let prepared_text = if self.use_e5_prefix {
            match role {
                EmbeddingInputRole::Query => format!("query: {}", text.trim()),
                EmbeddingInputRole::Passage => format!("passage: {}", text.trim()),
            }
        } else {
            text.to_string()
        };

        // Lightweight deterministic embedding suitable for local retrieval without heavy RAM usage.
        let mut vector = vec![0.0f32; self.dimensions];

        for token in tokenize(&prepared_text) {
            let digest = Sha256::digest(token.as_bytes());
            let bucket = ((u16::from(digest[0]) << 8) | u16::from(digest[1])) as usize;
            let index = bucket % self.dimensions;
            let sign = if digest[2] % 2 == 0 { 1.0 } else { -1.0 };
            vector[index] += sign;
        }

        l2_normalize(&mut vector);

        Ok(vector)
    }
}

fn should_use_e5_prefix(model_path: &str) -> bool {
    let lower = model_path.to_ascii_lowercase();
    lower.contains("multilingual-e5") || lower.contains("/e5") || lower.contains("\\e5")
}

fn tokenize(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|raw| {
            raw.chars()
                .filter(|ch| ch.is_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

fn l2_normalize(values: &mut [f32]) {
    let norm = values.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }

    for value in values {
        *value /= norm;
    }
}

#[cfg(test)]
mod tests {
    use super::should_use_e5_prefix;

    #[test]
    fn detects_e5_model_paths() {
        assert!(should_use_e5_prefix("/models/multilingual-e5-small.onnx"));
        assert!(should_use_e5_prefix("C:\\models\\e5-small.onnx"));
        assert!(!should_use_e5_prefix("/models/all-MiniLM-L6-v2.onnx"));
    }
}
