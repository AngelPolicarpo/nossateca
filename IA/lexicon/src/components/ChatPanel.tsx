import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { marked } from "marked";

type ChatMessage = {
  id: string;
  session_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  source_level?: string | null;
  source_label?: string;
  created_at: string;
  isTyping?: boolean;
};

type ChatAnswer = {
  answer: string;
  sourceLevel: string;
  sourceLabel: string;
};

type ChatPanelProps = {
  bookId: string;
  embedded?: boolean;
  prefillMessage?: {
    id: number;
    text: string;
  } | null;
};

export function ChatPanel({ bookId, embedded = false, prefillMessage = null }: ChatPanelProps) {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retryMessage, setRetryMessage] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  const sessionId = useMemo(() => `book-${bookId}-default`, [bookId]);

  useEffect(() => {
    const loadHistory = async () => {
      try {
        const result = await invoke<ChatMessage[]>("get_chat_history", { sessionId });
        setMessages(result);
      } catch {
        setMessages([]);
      }
    };

    void loadHistory();
  }, [sessionId]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, isGenerating]);

  useEffect(() => {
    if (!prefillMessage || prefillMessage.text.trim().length === 0) {
      return;
    }

    setInput(prefillMessage.text);
  }, [prefillMessage]);

  const sourceLabelFromLevel = (sourceLevel?: string | null) => {
    if (!sourceLevel) {
      return null;
    }

    if (sourceLevel === "level1") {
      return "Resposta baseada em trechos específicos do livro";
    }

    if (sourceLevel === "level2") {
      return "Resumo de capítulo";
    }

    if (sourceLevel === "level3") {
      return "Visão geral do livro";
    }

    return null;
  };

  const getErrorMessage = (err: unknown): string => {
    if (err instanceof Error && err.message.trim().length > 0) {
      return err.message;
    }

    if (typeof err === "string" && err.trim().length > 0) {
      return err;
    }

    if (
      typeof err === "object" &&
      err !== null &&
      "message" in err &&
      typeof (err as { message?: unknown }).message === "string"
    ) {
      const message = (err as { message: string }).message.trim();
      if (message.length > 0) {
        return message;
      }
    }

    return "Falha ao gerar resposta";
  };

  const sendMessage = async (messageOverride?: string) => {
    const question = (messageOverride ?? input).trim();
    if (!question || isGenerating) {
      return;
    }

    const isRetry = typeof messageOverride === "string";
    const typingId = `typing-${crypto.randomUUID()}`;

    if (!isRetry) {
      const userMessage: ChatMessage = {
        id: crypto.randomUUID(),
        session_id: sessionId,
        role: "user",
        content: question,
        created_at: new Date().toISOString(),
      };

      setMessages((prev) => [...prev, userMessage]);
      setInput("");
    }

    const typingMessage: ChatMessage = {
      id: typingId,
      session_id: sessionId,
      role: "assistant",
      content: "Digitando...",
      created_at: new Date().toISOString(),
      isTyping: true,
    };

    setMessages((prev) => [...prev, typingMessage]);
    setIsGenerating(true);
    setRetryMessage(null);
    setError(null);

    try {
      const answerPayload = await invoke<ChatAnswer>("chat_with_book", {
        bookId,
        message: question,
        sessionId,
      });

      const assistantMessage: ChatMessage = {
        id: crypto.randomUUID(),
        session_id: sessionId,
        role: "assistant",
        content: answerPayload.answer,
        source_level: answerPayload.sourceLevel,
        source_label: answerPayload.sourceLabel,
        created_at: new Date().toISOString(),
      };

      setMessages((prev) => {
        const withoutTyping = prev.filter((message) => message.id !== typingId);
        return [...withoutTyping, assistantMessage];
      });
    } catch (err) {
      const message = getErrorMessage(err);
      const friendlyMessage =
        message.includes("timed out") || message.includes("timeout")
          ? "A geração demorou demais. Tente novamente em alguns segundos."
          : `Não foi possível gerar resposta agora. ${message}`;

      setMessages((prev) => prev.filter((msg) => msg.id !== typingId));
      setError(friendlyMessage);
      setRetryMessage(question);
    } finally {
      setIsGenerating(false);
    }
  };

  return (
    <aside className={`chat-panel ${embedded ? "embedded" : ""}`.trim()}>
      <h3>Chat com livro</h3>
      {error && <p className="chat-error">{error}</p>}
      {error && retryMessage && (
        <button
          type="button"
          className="secondary-button chat-retry"
          onClick={() => void sendMessage(retryMessage)}
          disabled={isGenerating}
        >
          Tentar novamente
        </button>
      )}
      {isGenerating && <p className="chat-status">Pensando... Assistente está digitando.</p>}

      <div className="chat-messages">
        {messages.length === 0 && <p className="chat-empty">Sem mensagens ainda.</p>}
        {messages.map((message) => (
          <article
            key={message.id}
            className={`chat-message ${message.role}${message.isTyping ? " typing" : ""}`}
          >
            <strong>{message.role === "user" ? "Você" : "Assistente"}</strong>
            {message.role === "assistant" && !message.isTyping && (
              <p className="chat-source-label">
                {message.source_label ?? sourceLabelFromLevel(message.source_level)}
              </p>
            )}
            {message.role === "assistant" && !message.isTyping ? (
              <div
                className="chat-markdown"
                dangerouslySetInnerHTML={{ __html: marked.parse(message.content) }}
              />
            ) : (
              <p>{message.content}</p>
            )}
          </article>
        ))}
        <div ref={messagesEndRef} />
      </div>

      <div className="chat-input-row">
        <textarea
          value={input}
          onChange={(event) => setInput(event.currentTarget.value)}
          onKeyDown={(event) => {
            if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
              event.preventDefault();
              void sendMessage();
            }
          }}
          placeholder="Pergunte algo sobre o livro..."
          rows={3}
        />
        <button
          type="button"
          className="primary-button"
          onClick={() => void sendMessage()}
          disabled={isGenerating || !input.trim()}
        >
          {isGenerating ? "Gerando..." : "Enviar"}
        </button>
      </div>
    </aside>
  );
}
