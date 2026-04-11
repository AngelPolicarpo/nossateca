use std::num::NonZeroU32;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use anyhow::{Context, Result};

use candle_core as _;
use candle_nn as _;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use llama_cpp_2::{LlamaCppError, TokenToStringError};

use crate::ai::output_sanitizer::{
    config_for_model, sanitize_output as sanitize_with_config, OutputSanitizerConfig,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ContextSizing {
    n_ctx: u32,
    n_batch: u32,
    n_ubatch: u32,
}

#[derive(Clone)]
pub struct LlmEngine {
    model_path: String,
    model: Arc<Mutex<Option<CachedModel>>>,
}

struct CachedModel {
    n_gpu_layers: u32,
    main_gpu: Option<i32>,
    model: LlamaModel,
}

impl LlmEngine {
    pub fn new(model_path: &str) -> Result<Self> {
        if model_path.trim().is_empty() {
            return Err(anyhow::anyhow!("LLM model path is not configured"));
        }

        if !Path::new(model_path).exists() {
            return Err(anyhow::anyhow!("LLM model not found at {}", model_path));
        }

        Ok(Self {
            model_path: model_path.to_string(),
            model: Arc::new(Mutex::new(None)),
        })
    }

    pub fn generate(&self, prompt: &str, max_tokens: i32) -> Result<String> {
        const REPETITION_PENALTY_LAST_N: i32 = 256;
        const REPETITION_PENALTY: f32 = 1.20;

        if prompt.trim().is_empty() {
            anyhow::bail!("Prompt vazio para geração de resposta");
        }

        let generation_prompt = prompt.to_string();

        let _generation_guard = generation_lock()
            .lock()
            .map_err(|_| anyhow::anyhow!("Falha ao sincronizar geração local"))?;

        let backend = init_backend()?;
        let n_gpu_layers = read_env_u32("LEXICON_N_GPU_LAYERS").unwrap_or(40);
        let main_gpu = read_env_i32("LEXICON_MAIN_GPU");

        let mut model_guard = self
            .model
            .lock()
            .map_err(|_| anyhow::anyhow!("Falha ao sincronizar cache de modelo local"))?;

        let should_reload_model = model_guard.as_ref().is_none_or(|cached| {
            cached.n_gpu_layers != n_gpu_layers || cached.main_gpu != main_gpu
        });

        if should_reload_model {
            let loaded_model = load_model(backend, &self.model_path, n_gpu_layers, main_gpu)?;
            *model_guard = Some(CachedModel {
                n_gpu_layers,
                main_gpu,
                model: loaded_model,
            });
        }

        let model = &model_guard
            .as_ref()
            .expect("modelo deve estar carregado após validação")
            .model;

        let threads = std::thread::available_parallelism()
            .map(|n| n.get() as i32)
            .unwrap_or(4)
            .clamp(1, 16);

        let n_ctx = read_env_u32("LEXICON_N_CTX")
            .unwrap_or(2048)
            .clamp(512, 8192);
        let n_batch = read_env_u32("LEXICON_N_BATCH")
            .unwrap_or(512)
            .clamp(32, n_ctx);
        let n_ubatch = read_env_u32("LEXICON_N_UBATCH")
            .unwrap_or(128)
            .clamp(16, n_batch);
        let requested_context = ContextSizing {
            n_ctx,
            n_batch,
            n_ubatch,
        };
        let fallback_context = ContextSizing {
            n_ctx: requested_context.n_ctx.min(1024),
            n_batch: requested_context.n_batch.min(256),
            n_ubatch: requested_context.n_ubatch.min(64),
        };

        let mut active_context = requested_context;
        let mut context = match model
            .new_context(backend, context_params_for(requested_context, threads))
        {
            Ok(ctx) => ctx,
            Err(primary_err) => {
                let primary_message = primary_err.to_string();
                if fallback_context == requested_context || !is_allocation_error(&primary_message) {
                    return Err(anyhow::anyhow!(
                        "Falha ao criar contexto do modelo (n_gpu_layers={n_gpu_layers}, n_ctx={}, n_batch={}, n_ubatch={}): {primary_message}",
                        requested_context.n_ctx,
                        requested_context.n_batch,
                        requested_context.n_ubatch
                    ));
                }

                active_context = fallback_context;
                model
                    .new_context(backend, context_params_for(fallback_context, threads))
                    .with_context(|| {
                        format!(
                            "Falha ao criar contexto mesmo em modo reduzido (n_gpu_layers={n_gpu_layers}, n_ctx={}, n_batch={}, n_ubatch={}). Tente reduzir LEXICON_N_GPU_LAYERS",
                            fallback_context.n_ctx,
                            fallback_context.n_batch,
                            fallback_context.n_ubatch
                        )
                    })?
            }
        };

        let mut prompt_tokens = model
            .str_to_token(&generation_prompt, AddBos::Always)
            .context("Falha ao tokenizar prompt")?;

        if prompt_tokens.is_empty() {
            anyhow::bail!("Prompt sem tokens válidos");
        }

        let requested_generation_steps =
            usize::try_from(max_tokens.max(1)).unwrap_or(256).min(1024);
        let min_generation_steps = read_env_u32("LEXICON_MIN_GENERATION_TOKENS")
            .unwrap_or(96)
            .clamp(16, 512) as usize;

        let n_ctx_tokens = usize::try_from(active_context.n_ctx).unwrap_or(4096);
        let token_safety_margin = 8usize;
        let target_generation_steps = requested_generation_steps.max(min_generation_steps);
        let max_prompt_tokens = n_ctx_tokens
            .saturating_sub(target_generation_steps)
            .saturating_sub(token_safety_margin)
            .max(1);

        if prompt_tokens.len() > max_prompt_tokens {
            let keep_from = prompt_tokens.len() - max_prompt_tokens;
            prompt_tokens = prompt_tokens.split_off(keep_from);
        }

        let max_generation_capacity = n_ctx_tokens
            .saturating_sub(prompt_tokens.len())
            .saturating_sub(token_safety_margin)
            .max(1);

        let generation_steps = requested_generation_steps.min(max_generation_capacity);
        let n_batch_tokens = usize::try_from(active_context.n_batch)
            .unwrap_or(512)
            .max(1);
        let batch_capacity = n_batch_tokens.saturating_add(8);
        let mut batch = LlamaBatch::new(batch_capacity, 1);

        let mut prompt_cursor = 0usize;
        while prompt_cursor < prompt_tokens.len() {
            let prompt_end = (prompt_cursor + n_batch_tokens).min(prompt_tokens.len());
            batch.clear();

            for index in prompt_cursor..prompt_end {
                let token_position = i32::try_from(index).unwrap_or(i32::MAX.saturating_sub(1));
                let is_last_prompt_token = index + 1 == prompt_tokens.len();
                batch
                    .add(
                        prompt_tokens[index],
                        token_position,
                        &[0],
                        is_last_prompt_token,
                    )
                    .context("Falha ao preparar lote de prompt")?;
            }

            context
                .decode(&mut batch)
                .context("Falha ao decodificar lote do prompt no modelo")?;
            prompt_cursor = prompt_end;
        }

        let mut response = String::new();
        let mut position = i32::try_from(prompt_tokens.len()).unwrap_or(i32::MAX.saturating_sub(1));
        let mut repetition_sampler =
            LlamaSampler::penalties(REPETITION_PENALTY_LAST_N, REPETITION_PENALTY, 0.0, 0.0);
        repetition_sampler.accept_many(prompt_tokens.iter());

        for _ in 0..generation_steps {
            let mut candidates = context.token_data_array();
            repetition_sampler.apply(&mut candidates);
            let token = candidates.sample_token_greedy();

            if model.is_eog_token(token) || token == model.token_eos() {
                break;
            }

            response.push_str(&decode_token_piece(&model, token)?);

            batch.clear();
            batch
                .add(token, position, &[0], true)
                .context("Falha ao adicionar token gerado no batch")?;
            context
                .decode(&mut batch)
                .context("Falha ao decodificar token gerado")?;

            repetition_sampler.accept(token);
            position = position.saturating_add(1);
        }

        let cleaned = sanitize_output_for_model(&response, &self.model_path);
        if cleaned.is_empty() {
            anyhow::bail!("Modelo não retornou resposta útil");
        }

        Ok(cleaned)
    }
}

#[allow(dead_code)]
pub fn sanitize_output(raw: &str) -> String {
    sanitize_with_config(
        raw,
        OutputSanitizerConfig {
            requires_thinking_filter: true,
        },
    )
}

fn sanitize_output_for_model(raw: &str, model_path: &str) -> String {
    let config = config_for_model(model_path);
    sanitize_with_config(raw, config)
}

fn generation_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn init_backend() -> Result<&'static LlamaBackend> {
    static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

    if let Some(backend) = BACKEND.get() {
        return Ok(backend);
    }

    match LlamaBackend::init() {
        Ok(backend) => Ok(BACKEND.get_or_init(|| backend)),
        Err(LlamaCppError::BackendAlreadyInitialized) => BACKEND.get().ok_or_else(|| {
            anyhow::anyhow!(
                "Backend llama.cpp já inicializado externamente e indisponível no cache local"
            )
        }),
        Err(err) => Err(anyhow::anyhow!(
            "Falha ao inicializar backend llama.cpp: {err}"
        )),
    }
}

fn load_model(
    backend: &LlamaBackend,
    model_path: &str,
    n_gpu_layers: u32,
    main_gpu: Option<i32>,
) -> Result<LlamaModel> {
    let mut model_params = LlamaModelParams::default().with_n_gpu_layers(n_gpu_layers);

    if let Some(main_gpu) = main_gpu {
        model_params = model_params.with_main_gpu(main_gpu);
    }

    LlamaModel::load_from_file(backend, Path::new(model_path), &model_params)
        .with_context(|| format!("Falha ao carregar modelo GGUF em {model_path}"))
}

fn read_env_u32(name: &str) -> Option<u32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u32>().ok())
}

fn read_env_i32(name: &str) -> Option<i32> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<i32>().ok())
}

fn context_params_for(sizing: ContextSizing, threads: i32) -> LlamaContextParams {
    LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(sizing.n_ctx))
        .with_n_batch(sizing.n_batch)
        .with_n_ubatch(sizing.n_ubatch)
        .with_n_threads(threads)
        .with_n_threads_batch(threads)
}

fn is_allocation_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("out of memory")
        || message.contains("failed to allocate")
        || message.contains("alloc_buffer")
        || message.contains("gallocr")
}

fn decode_token_piece(model: &LlamaModel, token: LlamaToken) -> Result<String> {
    let bytes = match model.token_to_piece_bytes(token, 32, false, None) {
        Ok(bytes) => bytes,
        Err(TokenToStringError::InsufficientBufferSpace(required)) => {
            let required = usize::try_from(-required).unwrap_or(256);
            model
                .token_to_piece_bytes(token, required, false, None)
                .context("Falha ao decodificar token gerado")?
        }
        Err(err) => {
            return Err(anyhow::anyhow!(
                "Falha ao converter token para texto: {err}"
            ))
        }
    };

    Ok(String::from_utf8_lossy(&bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::{sanitize_output, sanitize_output_for_model};

    #[test]
    fn removes_think_tags() {
        let raw = "<think>cadeia interna</think>Resposta final limpa";
        assert_eq!(sanitize_output(raw), "Resposta final limpa");
    }

    #[test]
    fn keeps_only_response_after_final_answer_marker() {
        let raw =
            "Thinking Process: passo 1\\nAnalyze the Request: detalhe\\nFinal Answer: resultado";
        assert_eq!(sanitize_output(raw), "resultado");
    }

    #[test]
    fn keeps_only_response_after_response_marker() {
        let raw =
            "Role: analyst\\nContext: chapter review\\nTask: answer\\nResponse: resposta objetiva";
        assert_eq!(sanitize_output(raw), "resposta objetiva");
    }

    #[test]
    fn removes_token_count_tail_when_filtering() {
        let raw = "Resumo objetivo do capítulo.\\nToken count check: ~90 tokens\\nWait, I need to check the input again.";
        let cleaned = sanitize_output_for_model(raw, "/models/qwen-3.gguf");
        assert_eq!(cleaned, "Resumo objetivo do capítulo.");
    }

    #[test]
    fn preserves_text_when_no_reliable_marker_exists() {
        let raw = "Analyze the Request: detalhe interno\\nFormulate the Answer: rascunho\\nTexto final sem marcador";
        let expected = "Analyze the Request: detalhe interno\nFormulate the Answer: rascunho\nTexto final sem marcador";
        assert_eq!(sanitize_output(raw), expected);
    }

    #[test]
    fn preserves_original_when_no_markers() {
        let raw = "Texto normal de outro modelo";
        assert_eq!(sanitize_output(raw), "Texto normal de outro modelo");
    }

    #[test]
    fn keeps_text_for_models_without_filter_requirement() {
        let raw = "Texto final.\\nToken count check: 40";
        let cleaned = sanitize_output_for_model(raw, "/models/llama-3.1.gguf");
        assert_eq!(cleaned, raw);
    }
}
