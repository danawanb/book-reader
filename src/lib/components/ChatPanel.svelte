<script lang="ts">
  import { chat, type Message } from "../api/openai";
  import type { Book } from "../stores/books";

  let { book, selectedText }: {
    book: Book;
    selectedText: string;
  } = $props();

  let messages = $state<Message[]>([]);
  let input = $state("");
  let streaming = $state(false);
  let error = $state("");
  let chatContainer: HTMLDivElement;

  $effect(() => {
    if (selectedText) {
      input = `Please explain the following text:\n\n"${selectedText}"`;
    }
  });

  async function send() {
    const userText = input.trim();
    if (!userText || streaming) return;

    error = "";
    input = "";

    const systemMsg: Message = {
      role: "system",
      content: `You are a reading assistant. The user is reading "${book.title}"${book.author ? ` by ${book.author}` : ""}. Help explain content, words, or concepts they ask about. Reply in the same language as the user's question.`,
    };

    messages = [...messages, { role: "user", content: userText }];
    messages = [...messages, { role: "assistant", content: "" }];
    const assistantIdx = messages.length - 1;

    streaming = true;
    try {
      await chat([systemMsg, ...messages.slice(0, -1)], (chunk) => {
        messages = messages.map((m, i) =>
          i === assistantIdx ? { ...m, content: m.content + chunk } : m
        );
        scrollBottom();
      });
    } catch (e) {
      error = String(e);
      messages = messages.slice(0, -1);
    } finally {
      streaming = false;
      scrollBottom();
    }
  }

  function scrollBottom() {
    requestAnimationFrame(() => {
      if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
    });
  }

  function clearChat() {
    messages = [];
    error = "";
  }
</script>

<div class="chat-panel">
  {#if messages.length > 0}
    <button class="clear-btn" onclick={clearChat}>Clear chat</button>
  {/if}

  <div class="messages" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="hint">
        <p>Select text from the book to ask AI directly, or type your question.</p>
      </div>
    {/if}
    {#each messages as msg}
      <div class="msg {msg.role}">
        <div class="bubble">{msg.content}</div>
      </div>
    {/each}
    {#if streaming && messages.at(-1)?.content === ""}
      <div class="typing">...</div>
    {/if}
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="input-row">
    <textarea
      placeholder="Ask anything about this book..."
      bind:value={input}
      rows="3"
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          send();
        }
      }}
    ></textarea>
    <button onclick={send} disabled={streaming || !input.trim()}>
      {streaming ? "..." : "Send"}
    </button>
  </div>
</div>

<style>
  .chat-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e2e;
    position: relative;
  }
  .clear-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    background: none;
    border: none;
    color: #6c7086;
    font-size: 11px;
    cursor: pointer;
    z-index: 1;
  }
  .clear-btn:hover {
    color: #f38ba8;
  }
  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .hint {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 20px;
  }
  .hint p {
    color: #6c7086;
    font-size: 13px;
    text-align: center;
    line-height: 1.6;
  }
  .msg {
    display: flex;
  }
  .msg.user {
    justify-content: flex-end;
  }
  .msg.assistant {
    justify-content: flex-start;
  }
  .bubble {
    max-width: 90%;
    padding: 10px 14px;
    border-radius: 12px;
    font-size: 13px;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .msg.user .bubble {
    background: #89b4fa;
    color: #1e1e2e;
    border-bottom-right-radius: 4px;
  }
  .msg.assistant .bubble {
    background: #313244;
    color: #cdd6f4;
    border-bottom-left-radius: 4px;
  }
  .typing {
    color: #6c7086;
    font-size: 20px;
    letter-spacing: 4px;
    padding-left: 12px;
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.3; }
  }
  .error {
    background: rgba(243, 139, 168, 0.15);
    border-top: 1px solid #f38ba8;
    color: #f38ba8;
    padding: 8px 12px;
    font-size: 12px;
  }
  .input-row {
    display: flex;
    gap: 8px;
    padding: 10px;
    border-top: 1px solid #313244;
    flex-shrink: 0;
  }
  textarea {
    flex: 1;
    background: #313244;
    border: 1px solid #45475a;
    color: #cdd6f4;
    border-radius: 8px;
    padding: 8px 12px;
    font-size: 13px;
    resize: none;
    font-family: inherit;
    outline: none;
    line-height: 1.5;
  }
  textarea:focus {
    border-color: #89b4fa;
  }
  .input-row button {
    background: #89b4fa;
    color: #1e1e2e;
    border: none;
    border-radius: 8px;
    padding: 0 16px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    align-self: flex-end;
    height: 36px;
    transition: background 0.1s;
  }
  .input-row button:hover:not(:disabled) {
    background: #74c7ec;
  }
  .input-row button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
