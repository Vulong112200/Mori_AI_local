import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import * as api from "./api";
import type { LlmStatus, Message } from "./api";

function statusLabel(s: LlmStatus): string {
  switch (s.state) {
    case "starting":
      return "Mori đang thức dậy…";
    case "ready":
      return "Sẵn sàng";
    case "restarting":
      return `Đang khởi động lại (lần ${s.attempt})…`;
    case "error":
      return `Lỗi: ${s.message}`;
    case "stopped":
      return "Đã dừng";
  }
}

const statusColor: Record<LlmStatus["state"], string> = {
  starting: "bg-amber-400",
  ready: "bg-emerald-500",
  restarting: "bg-amber-400",
  error: "bg-rose-500",
  stopped: "bg-slate-400",
};

export default function App() {
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [draft, setDraft] = useState("");
  const [streaming, setStreaming] = useState<string | null>(null); // null = không sinh
  const [status, setStatus] = useState<LlmStatus>({ state: "starting", model: "" });
  const [fastMode, setFastModeState] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    api.onLlmStatus(setStatus).then((u) => (unlisten = u));
    api
      .init()
      .then((s) => {
        setConversationId(s.conversation_id);
        setMessages(s.messages);
        setStatus(s.status);
        setFastModeState(s.fast_mode);
      })
      .catch((e) => setError(String(e)));
    return () => unlisten?.();
  }, []);

  useEffect(() => bottomRef.current?.scrollIntoView({ behavior: "smooth" }), [messages, streaming]);

  const generating = streaming !== null;
  const ready = status.state === "ready";

  async function send(e?: FormEvent) {
    e?.preventDefault();
    const text = draft.trim();
    if (!text || !conversationId || generating || !ready) return;
    setDraft("");
    setError(null);
    const pending: Message = { id: `pending-${Date.now()}`, role: "user", content: text, created_at: "" };
    setMessages((m) => [...m, pending]);
    setStreaming("");
    try {
      const saved = await api.sendMessage(conversationId, text, (ev) => {
        if (ev.type === "delta") setStreaming((s) => (s ?? "") + ev.text);
        else if (ev.message) setMessages((m) => [...m, ev.message!]);
      });
      setMessages((m) => m.map((x) => (x.id === pending.id ? saved : x)));
    } catch (err) {
      setError(String(err));
    } finally {
      setStreaming(null);
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) send();
  }

  async function startNew() {
    if (generating) return;
    setConversationId(await api.newConversation());
    setMessages([]);
  }

  async function toggleFast() {
    const next = !fastMode;
    setFastModeState(next);
    await api.setFastMode(next).catch((e) => setError(String(e)));
  }

  return (
    <div className="flex h-screen flex-col bg-gradient-to-b from-pink-50 to-violet-50 text-slate-800">
      <header className="flex items-center gap-3 border-b border-pink-100 bg-white/70 px-4 py-3 backdrop-blur">
        <div aria-hidden className="grid size-9 place-items-center rounded-full bg-pink-200 text-lg">
          🌸
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="font-semibold leading-tight">Mori</h1>
          <p className="flex items-center gap-1.5 text-xs text-slate-500" role="status">
            <span className={`size-2 rounded-full ${statusColor[status.state]}`} />
            <span className="truncate">{statusLabel(status)}</span>
          </p>
        </div>
        <label className="flex cursor-pointer items-center gap-1.5 text-xs text-slate-600" title="Dùng model nhỏ, trả lời nhanh hơn">
          <input type="checkbox" checked={fastMode} onChange={toggleFast} disabled={generating} className="accent-pink-500" />
          Chế độ nhanh
        </label>
        <button
          onClick={startNew}
          disabled={generating}
          className="rounded-lg px-2.5 py-1.5 text-xs text-slate-600 hover:bg-pink-100 disabled:opacity-40"
        >
          Cuộc trò chuyện mới
        </button>
      </header>

      <main className="flex-1 space-y-3 overflow-y-auto px-4 py-4" aria-live="polite">
        {messages.length === 0 && !generating && (
          <p className="mt-16 text-center text-sm text-slate-400">Chào bạn! Mình là Mori. Hôm nay bạn muốn nói chuyện gì nè?</p>
        )}
        {messages.map((m) => (
          <Bubble key={m.id} role={m.role} text={m.content} />
        ))}
        {generating && <Bubble role="assistant" text={streaming || "…"} />}
        <div ref={bottomRef} />
      </main>

      {error && (
        <p role="alert" className="mx-4 mb-2 rounded-lg bg-rose-50 px-3 py-2 text-xs text-rose-700">
          {error}
        </p>
      )}

      <form onSubmit={send} className="flex items-end gap-2 border-t border-pink-100 bg-white/70 p-3">
        <label htmlFor="draft" className="sr-only">
          Tin nhắn cho Mori
        </label>
        <textarea
          id="draft"
          rows={2}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={ready ? "Nhắn cho Mori… (Enter để gửi, Shift+Enter xuống dòng)" : "Đợi Mori sẵn sàng…"}
          className="flex-1 resize-none rounded-xl border border-pink-200 bg-white px-3 py-2 text-sm outline-none focus:border-pink-400"
        />
        {generating ? (
          <button type="button" onClick={() => api.stopGeneration()} className="rounded-xl bg-slate-700 px-4 py-2 text-sm text-white hover:bg-slate-800">
            Dừng
          </button>
        ) : (
          <button
            type="submit"
            disabled={!draft.trim() || !ready}
            className="rounded-xl bg-pink-500 px-4 py-2 text-sm text-white hover:bg-pink-600 disabled:opacity-40"
          >
            Gửi
          </button>
        )}
      </form>
    </div>
  );
}

function Bubble({ role, text }: { role: Message["role"]; text: string }) {
  const mine = role === "user";
  return (
    <div className={`flex ${mine ? "justify-end" : "justify-start"}`}>
      <div
        className={`max-w-[85%] whitespace-pre-wrap rounded-2xl px-3.5 py-2 text-sm shadow-sm ${
          mine ? "rounded-br-md bg-pink-500 text-white" : "rounded-bl-md bg-white text-slate-800"
        }`}
      >
        {text}
      </div>
    </div>
  );
}
