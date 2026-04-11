#[derive(Debug, Clone, Copy)]
pub struct OutputSanitizerConfig {
    pub requires_thinking_filter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineState {
    SeekingAnswer,
    CollectingAnswer,
}

pub fn config_for_model(model_path: &str) -> OutputSanitizerConfig {
    let lower = model_path.to_ascii_lowercase();
    // Qwen2.5 Instruct does not expose the reasoning mode by default.
    // Keep filter enabled only for model families commonly emitting reasoning traces.
    let requires_thinking_filter = lower.contains("deepseek-r1")
        || lower.contains("qwen3")
        || lower.contains("qwen-3")
        || lower.contains("qwq");

    OutputSanitizerConfig {
        requires_thinking_filter,
    }
}

pub fn sanitize_output(raw: &str, config: OutputSanitizerConfig) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if !config.requires_thinking_filter {
        return trimmed.to_string();
    }

    let normalized = normalize_line_breaks(trimmed);

    let without_think = strip_think_blocks(&normalized);
    let filtered = filter_reasoning_lines(&without_think);
    if filtered.trim().is_empty() {
        // Fail-safe: never drop the whole answer if the filter gets too aggressive.
        return normalized;
    }

    filtered.trim().to_string()
}

fn normalize_line_breaks(input: &str) -> String {
    input.replace("\\n", "\n")
}

fn strip_think_blocks(input: &str) -> String {
    let mut cursor = 0usize;
    let mut output = String::with_capacity(input.len());

    while let Some(open_start) = find_ascii_case_insensitive(input, "<think", cursor) {
        output.push_str(&input[cursor..open_start]);

        let Some(open_end_rel) = input[open_start..].find('>') else {
            return output.trim().to_string();
        };
        let open_end = open_start + open_end_rel + 1;

        let Some(close_start) = find_ascii_case_insensitive(input, "</think>", open_end) else {
            // If the model started a think block but never closed it, drop the tail fail-safe.
            return output.trim().to_string();
        };

        cursor = close_start + "</think>".len();
    }

    output.push_str(&input[cursor..]);
    output
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    if needle.is_empty() || start >= haystack.len() {
        return None;
    }

    let needle_bytes = needle.as_bytes();
    let haystack_bytes = haystack.as_bytes();
    if needle_bytes.len() > haystack_bytes.len() {
        return None;
    }

    let mut idx = start;
    while idx + needle_bytes.len() <= haystack_bytes.len() {
        let mut matched = true;
        for (offset, expected) in needle_bytes.iter().enumerate() {
            let got = haystack_bytes[idx + offset];
            if !got.eq_ignore_ascii_case(expected) {
                matched = false;
                break;
            }
        }

        if matched {
            return Some(idx);
        }

        idx += 1;
    }

    None
}

fn filter_reasoning_lines(input: &str) -> String {
    let mut state = LineState::SeekingAnswer;
    let mut answer_lines: Vec<String> = Vec::new();
    let has_explicit_answer_marker = contains_explicit_answer_marker(input);

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            if matches!(state, LineState::CollectingAnswer) {
                answer_lines.push(String::new());
            }
            continue;
        }

        let lower = line.to_lowercase();

        if let Some(rest) = strip_answer_marker(line, &lower) {
            if !rest.trim().is_empty() {
                answer_lines.push(rest.trim().to_string());
                state = LineState::CollectingAnswer;
            }
            continue;
        }

        if is_reasoning_marker(&lower) {
            match state {
                LineState::SeekingAnswer => {
                    if has_explicit_answer_marker {
                        continue;
                    }
                }
                LineState::CollectingAnswer => break,
            }
        }

        answer_lines.push(line.to_string());
        state = LineState::CollectingAnswer;
    }

    answer_lines.join("\n").trim().to_string()
}

fn contains_explicit_answer_marker(input: &str) -> bool {
    input.lines().any(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return false;
        }

        let lower = trimmed.to_lowercase();
        strip_answer_marker(trimmed, &lower).is_some()
    })
}

fn strip_answer_marker<'a>(line: &'a str, lower_line: &str) -> Option<&'a str> {
    for marker in ["final answer:", "resposta final:", "response:", "answer:"] {
        if lower_line.starts_with(marker) {
            return line.get(marker.len()..);
        }
    }

    None
}

fn is_reasoning_marker(lower_line: &str) -> bool {
    const PREFIX_MARKERS: [&str; 45] = [
        "thinking process",
        "processo de pensamento",
        "analyze the request",
        "análise do pedido",
        "analise do pedido",
        "analyze the source text",
        "análise do texto fonte",
        "analise do texto fonte",
        "draft the summary",
        "rascunho do resumo",
        "internal monologue",
        "monólogo interno",
        "monologo interno",
        "role:",
        "papel:",
        "context:",
        "contexto:",
        "token count check",
        "contagem de tokens",
        "count:",
        "contagem:",
        "wait, i need",
        "espera, preciso",
        "the input provided",
        "a entrada fornecida",
        "the user wants",
        "o usuário quer",
        "o usuario quer",
        "however, i must",
        "porém, devo",
        "porem, devo",
        "constraints:",
        "restrições:",
        "restricoes:",
        "input:",
        "entrada:",
        "task:",
        "tarefa:",
        "language:",
        "idioma:",
        "let me think",
        "deixe-me pensar",
        "let's think",
        "vou pensar",
        "step-by-step",
    ];

    if lower_line.starts_with("<think") || lower_line.starts_with("</think") {
        return true;
    }

    if is_numbered_event_line(lower_line) {
        return true;
    }

    PREFIX_MARKERS
        .iter()
        .any(|prefix| lower_line.starts_with(prefix))
}

fn is_numbered_event_line(lower_line: &str) -> bool {
    for prefix in ["event ", "evento "] {
        let Some(rest) = lower_line.strip_prefix(prefix) else {
            continue;
        };

        let rest = rest.trim_start();
        let digit_end = rest
            .find(|ch: char| !ch.is_ascii_digit())
            .unwrap_or(rest.len());
        if digit_end == 0 {
            continue;
        }

        let suffix = rest[digit_end..].trim_start();
        if suffix.starts_with(':') || suffix.starts_with('-') {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{config_for_model, sanitize_output, OutputSanitizerConfig};

    #[test]
    fn enables_filter_for_reasoning_families_only() {
        assert!(config_for_model("/models/qwen3-8b.gguf").requires_thinking_filter);
        assert!(config_for_model("/models/deepseek-r1.gguf").requires_thinking_filter);
        assert!(
            !config_for_model("/models/qwen2.5-7b-instruct-q4_k_m.gguf").requires_thinking_filter
        );
        assert!(!config_for_model("/models/llama-3.1.gguf").requires_thinking_filter);
    }

    #[test]
    fn strips_think_block_and_keeps_final_answer() {
        let input = "<think>analysis</think>Resposta final limpa";
        let output = sanitize_output(
            input,
            OutputSanitizerConfig {
                requires_thinking_filter: true,
            },
        );

        assert_eq!(output, "Resposta final limpa");
    }

    #[test]
    fn strips_open_think_without_close() {
        let input = "Resumo inicial válido.\n<think>draft interno sem fechamento";
        let output = sanitize_output(
            input,
            OutputSanitizerConfig {
                requires_thinking_filter: true,
            },
        );

        assert_eq!(output, "Resumo inicial válido.");
    }

    #[test]
    fn removes_reasoning_tail_after_valid_answer() {
        let input = "Capítulo 9 descreve a entrada do diário de 14 de junho de 1942.\nToken count check: ~100\nWait, I need to check again.";
        let output = sanitize_output(
            input,
            OutputSanitizerConfig {
                requires_thinking_filter: true,
            },
        );

        assert_eq!(
            output,
            "Capítulo 9 descreve a entrada do diário de 14 de junho de 1942."
        );
    }

    #[test]
    fn keeps_raw_when_filter_disabled() {
        let input = "Resposta curta.\nToken count check: 40";
        let output = sanitize_output(
            input,
            OutputSanitizerConfig {
                requires_thinking_filter: false,
            },
        );

        assert_eq!(output, input);
    }

    #[test]
    fn filters_portuguese_reasoning_markers() {
        let input =
            "Processo de pensamento:\nAnálise do pedido: resumir\nResposta final: resumo limpo";
        let output = sanitize_output(
            input,
            OutputSanitizerConfig {
                requires_thinking_filter: true,
            },
        );

        assert_eq!(output, "resumo limpo");
    }

    #[test]
    fn does_not_filter_legitimate_event_word() {
        let input = "Event horizons são um conceito de física e não um rascunho interno.";
        let output = sanitize_output(
            input,
            OutputSanitizerConfig {
                requires_thinking_filter: true,
            },
        );

        assert_eq!(output, input);
    }
}
