# Feature: IA Local (Core Diferencial)

> **Contexto:** Chat contextual, tradução, TTS, RAG. Stack: llama-cpp-2, candle.

## 1. Visão Geral

**Nome:** AI Layer
**Descrição:** Inteligência artificial local para transformar leitura em experiência interativa
**Prioridade:** Alta (diferencial)
**Stack:** llama-cpp-2, candle, piper-rs

## 2. Funcionalidades

### Chat com Livro (RAG)
- [ ] Perguntas sobre conteúdo
- [ ] Contexto automático (chunks relevantes)
- [ ] Memória de conversa por livro
- [ ] Citação de fontes (posição no texto)

### Tradução
- [ ] Tradução contextual (não literal)
- [ ] Preservação de formatação
- [ ] Detecção automática de idioma

### TTS (Text-to-Speech)
- [ ] Voz natural offline (Piper)
- [ ] Leitura contínua
- [ ] Controle de velocidade
- [ ] Highlight síncrono da palavra

### Resumos
- [ ] Por capítulo
- [ ] Por seleção
- [ ] Formatos: bullet points, timeline

### Learning Mode
- [ ] Geração de flashcards
- [ ] Quiz automático
- [ ] Revisão espaçada integrada

## 3. Arquitetura

```
┌─────────────────────────────────────────┐
│  UI Components                          │
│  ├── ChatInterface                      │
│  ├── TranslationPopup                   │
│  ├── TtsControls                        │
│  └── FlashcardGenerator                 │
└─────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│  AI Service (Rust)                      │
│  ├── LlmEngine (llama-cpp-2)            │
│  ├── EmbeddingEngine (candle)           │
│  ├── RagEngine                          │
│  └── TtsEngine (piper-rs)               │
└─────────────────────────────────────────┘
```

## 4. Modelos

### LLM
- **Modelo:** Llama-2-7B ou Mistral-7B
- **Formato:** GGUF Q4_K_M (4-bit quantized)
- **Tamanho:** ~4GB
- **Contexto:** 4096 tokens

### Embeddings
- **Modelo:** all-MiniLM-L6-v2
- **Dimensão:** 384
- **Formato:** ONNX via `ort`

### TTS
- **Engine:** Piper
- **Vozes:** Vários idiomas disponíveis

## 5. API

```rust
// Chat
#[tauri::command]
pub async fn chat_with_book(
    book_id: String,
    message: String,
    session_id: Option<String>,
) -> Result<ChatResponse, String>;

// Streaming
#[tauri::command]
pub async fn chat_stream(
    book_id: String,
    message: String,
    window: Window,
) -> Result<(), String>;

// Tradução
#[tauri::command]
pub async fn translate_text(
    text: String,
    target_language: String,
) -> Result<String, String>;

// TTS
#[tauri::command]
pub async fn speak_text(
    text: String,
    voice_id: Option<String>,
) -> Result<(), String>;

#[tauri::command]
pub async fn stop_speaking() -> Result<(), String>;

// Flashcards
#[tauri::command]
pub async fn generate_flashcards(
    book_id: String,
    chapter_id: Option<String>,
) -> Result<Vec<Flashcard>, String>;
```

## 6. RAG Pipeline

```rust
pub struct RagEngine {
    llm: LlmEngine,
    embeddings: EmbeddingEngine,
}

impl RagEngine {
    pub async fn query(&self, book_id: &str, question: &str) -> Result<String> {
        // 1. Embed pergunta
        let query_emb = self.embeddings.embed(question).await?;
        
        // 2. Buscar chunks similares
        let chunks = search_similar_chunks(book_id, &query_emb, 5).await?;
        
        // 3. Construir prompt
        let context = chunks.iter().map(|c| &c.text).join("

");
        let prompt = format!(
            "Contexto:
{}

Pergunta: {}
Resposta:",
            context, question
        );
        
        // 4. Gerar resposta
        self.llm.generate(&prompt, 512).await
    }
}
```

## 7. Chunking Strategy

```rust
pub fn chunk_text(text: &str) -> Vec<Chunk> {
    // Por parágrafo com overlap
    // Max 512 tokens por chunk
    // Overlap de 50 tokens
}
```

## 8. Banco de Dados

- `book_chunks`: Texto segmentado
- `book_embeddings`: Vetores (sqlite-vec)
- `chat_sessions`: Histórico de conversas
- `chat_messages`: Mensagens individuais
- `flashcards`: Cartões gerados

## 9. Configuração

```rust
pub struct AiConfig {
    pub llm_model_path: String,
    pub embedding_model_path: String,
    pub tts_voice_path: String,
    pub context_window: usize,
    pub max_tokens: i32,
    pub temperature: f32,
}
```

## 10. Testes

- [ ] Resposta gerada em < 5s (GPU) / < 15s (CPU)
- [ ] Contexto relevante recuperado
- [ ] Tradução preserva markdown
- [ ] TTS não trava UI
