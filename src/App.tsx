import { useEffect, useRef, useState, type FormEvent, type KeyboardEvent } from "react";
import * as api from "./api";
import type { LlmStatus, Message, Mic, Sample } from "./api";

type MicState = "idle" | "listening" | "transcribing";

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
  const [voiceOut, setVoiceOutState] = useState(true);
  const [ttsVoice, setTtsVoiceState] = useState("maichi");
  const [ttsVoices, setTtsVoices] = useState<string[]>([]);
  const [speaking, setSpeaking] = useState(false);
  const [ttsStatus, setTtsStatus] = useState<"loading" | "ready" | "error" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const draftRef = useRef<HTMLTextAreaElement>(null);
  const [mics, setMics] = useState<Mic[]>([]);
  const [micId, setMicId] = useState<string | null>(null);
  const [micState, setMicStateRaw] = useState<MicState>("idle");
  // Ref để handler phím (đăng ký một lần) luôn thấy trạng thái mới nhất.
  const micRef = useRef<MicState>("idle");
  const micStarted = useRef<Promise<boolean>>(Promise.resolve(false));
  // Câu thu mẫu chốt lúc BẮT ĐẦU nói (đổi câu giữa chừng không làm lưu nhầm file).
  const recordingSample = useRef<Sample | undefined>(undefined);
  // Tự gửi: nói xong là gửi luôn (kiểu hội thoại). Tắt → transcript vào ô nhập để sửa trước.
  const [autoSend, setAutoSendRaw] = useState(() => {
    try {
      return localStorage.getItem("mori.autoSend") !== "0";
    } catch {
      return true;
    }
  });
  const autoSendRef = useRef(autoSend);
  const setAutoSend = (v: boolean) => {
    autoSendRef.current = v;
    setAutoSendRaw(v);
    try {
      localStorage.setItem("mori.autoSend", v ? "1" : "0");
    } catch {
      /* chỉ là tiện ích, không lưu được cũng không sao */
    }
  };
  // Thu mẫu (bản dev): null = tắt; F2 lưu câu đang hiện thay vì nhận dạng.
  const [samples, setSamples] = useState<Sample[] | null>(null);
  const [sampleIdx, setSampleIdx] = useState(0);
  const [lastSaved, setLastSaved] = useState<{ file: string; text: string; secs: number; heard: string } | null>(null);
  const setMicState = (m: MicState) => {
    micRef.current = m;
    setMicStateRaw(m);
  };

  useEffect(() => {
    const unlisten: (() => void)[] = [];
    api.onLlmStatus(setStatus).then((u) => unlisten.push(u));
    api.onTtsPlaying(setSpeaking).then((u) => unlisten.push(u));
    api.onTtsStatus(setTtsStatus).then((u) => unlisten.push(u));
    api
      .init()
      .then((s) => {
        setConversationId(s.conversation_id);
        setMessages(s.messages);
        setStatus(s.status);
        setFastModeState(s.fast_mode);
        setMicId(s.mic);
        setVoiceOutState(s.voice_out);
        setTtsVoiceState(s.tts_voice);
        setTtsVoices(s.tts_voices);
      })
      .catch((e) => setError(String(e)));
    api.listMics().then(setMics, () => {});
    return () => unlisten.forEach((u) => u());
  }, []);

  // Ngoặc nhọn: scrollIntoView trên WebView2 mới trả về Promise → không được trả nó cho useEffect.
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, streaming]);

  // Push-to-talk: giữ F2 (hoặc bấm nút micro) để nói, Esc để hủy.
  useEffect(() => {
    const down = (e: globalThis.KeyboardEvent) => {
      if (e.key === "F2" && !e.repeat) {
        e.preventDefault();
        startMic();
      } else if (e.key === "Escape" && micRef.current === "listening") {
        setMicState("idle");
        // Chờ start xong mới hủy, kẻo hủy chạy trước rồi micro mới mở.
        micStarted.current.then((ok) => {
          if (ok) api.cancelListening();
        });
      }
    };
    const up = (e: globalThis.KeyboardEvent) => {
      if (e.key === "F2") stopMic();
    };
    // Giữ F2 rồi chuyển cửa sổ → không nhận được keyup: dừng luôn.
    const blur = () => stopMic();
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);

  async function startMic() {
    if (micRef.current !== "idle") return;
    if (latest.current.sampling && !latest.current.sample) return; // đang thu mẫu nhưng hết câu: không lọt sang chat
    recordingSample.current = latest.current.sample;
    setMicState("listening");
    setError(null);
    micStarted.current = api.startListening().then(
      () => true,
      (e) => {
        setError(String(e));
        return false;
      },
    );
    if (!(await micStarted.current)) setMicState("idle");
  }

  async function stopMic() {
    if (micRef.current !== "listening") return;
    setMicState("transcribing");
    if (!(await micStarted.current)) return setMicState("idle"); // thả phím trước khi micro kịp mở
    try {
      const sample = recordingSample.current;
      if (sample) {
        const { secs, heard } = await api.saveSample(sample.file);
        setSamples((all) => all && all.map((x) => (x.file === sample.file ? { ...x, recorded: true } : x)));
        setSampleIdx((i) => i + 1);
        setLastSaved({ file: sample.file, text: sample.text, secs, heard });
        return;
      }
      const text = await api.stopListening();
      if (!text) setError("Mình chưa nghe thấy gì — thử nói to hơn hoặc kiểm tra micro nhé.");
      else if (autoSendRef.current && latest.current.canSend) latest.current.send(undefined, text);
      else setDraft((d) => (d.trim() ? `${d.trimEnd()} ${text}` : text)); // đang trả lời / tắt Tự gửi → để sửa
      draftRef.current?.focus();
    } catch (e) {
      setError(String(e));
    } finally {
      setMicState("idle");
    }
  }

  async function toggleVoiceOut() {
    const next = !voiceOut;
    setVoiceOutState(next);
    await api.setVoiceOut(next).catch((e) => {
      setVoiceOutState(!next);
      setError(String(e));
    });
  }

  async function chooseVoice(v: string) {
    setTtsVoiceState(v);
    await api.setTtsVoice(v).catch((e) => setError(String(e)));
  }

  async function chooseMic(id: string) {
    const next = id || null;
    setMicId(next);
    await api.setMic(next).catch((e) => setError(String(e)));
  }

  const generating = streaming !== null;
  const ready = status.state === "ready";

  async function send(e?: FormEvent, spoken?: string) {
    e?.preventDefault();
    const text = (spoken ?? draft).trim();
    if (!text || !conversationId || generating || !ready) return;
    if (spoken === undefined) setDraft("");
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

  // Handler phím đăng ký một lần → gọi bản send/điều kiện mới nhất qua ref.
  const latest = useRef<{ send: typeof send; canSend: boolean; sampling?: boolean; sample?: Sample }>({ send, canSend: false });
  const currentSample = samples?.[sampleIdx];
  latest.current = { send, canSend: !!conversationId && !generating && ready, sampling: !!samples, sample: currentSample };

  async function toggleSampling() {
    if (samples) return setSamples(null);
    try {
      const all = await api.listSamples();
      setSamples(all);
      setSampleIdx(Math.max(0, all.findIndex((x) => !x.recorded)));
    } catch (e) {
      setError(String(e));
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) send();
  }

  async function startNew() {
    if (generating) return;
    try {
      setConversationId(await api.newConversation());
      setMessages([]);
    } catch (e) {
      setError(String(e));
    }
  }

  async function toggleFast() {
    const next = !fastMode;
    setFastModeState(next);
    await api.setFastMode(next).catch((e) => {
      setFastModeState(!next); // backend không đổi → trả checkbox về như cũ
      setError(String(e));
    });
  }

  return (
    <div className="flex h-screen flex-col bg-gradient-to-b from-pink-50 to-violet-50 text-slate-800">
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2 border-b border-pink-100 bg-white/70 px-4 py-3 backdrop-blur">
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
        <label htmlFor="mic" className="sr-only">
          Micro
        </label>
        <select
          id="mic"
          value={micId ?? ""}
          onChange={(e) => chooseMic(e.target.value)}
          disabled={micState !== "idle"}
          title="Micro dùng để nói chuyện"
          className="max-w-28 truncate rounded-lg border border-pink-100 bg-white px-1.5 py-1 text-xs text-slate-600"
        >
          <option value="">Micro mặc định</option>
          {mics.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name}
            </option>
          ))}
        </select>
        <label
          className="flex cursor-pointer items-center gap-1.5 text-xs text-slate-600"
          title={ttsStatus === "loading" ? "Đang nạp giọng…" : ttsStatus === "error" ? "Giọng nói bị lỗi" : "Mori đọc câu trả lời thành tiếng"}
        >
          <input type="checkbox" checked={voiceOut} onChange={toggleVoiceOut} className="accent-pink-500" />
          Mori nói{voiceOut && ttsStatus === "loading" ? "…" : voiceOut && ttsStatus === "error" ? " ⚠" : ""}
        </label>
        {voiceOut && ttsVoices.length > 0 && (
          <>
            <label htmlFor="voice" className="sr-only">
              Giọng của Mori
            </label>
            <select
              id="voice"
              value={ttsVoice}
              onChange={(e) => chooseVoice(e.target.value)}
              title="Giọng của Mori"
              className="max-w-24 rounded-lg border border-pink-100 bg-white px-1.5 py-1 text-xs text-slate-600"
            >
              {ttsVoices.map((v) => (
                <option key={v} value={v}>
                  {v}
                </option>
              ))}
            </select>
          </>
        )}
        <label className="flex cursor-pointer items-center gap-1.5 text-xs text-slate-600" title="Nói xong gửi luôn, không cần bấm Gửi">
          <input type="checkbox" checked={autoSend} onChange={(e) => setAutoSend(e.target.checked)} className="accent-pink-500" />
          Tự gửi
        </label>
        <label className="flex cursor-pointer items-center gap-1.5 text-xs text-slate-600" title="Dùng model nhỏ, trả lời nhanh hơn">
          <input type="checkbox" checked={fastMode} onChange={toggleFast} disabled={generating} className="accent-pink-500" />
          Chế độ nhanh
        </label>
        {import.meta.env.DEV && (
          <button
            onClick={toggleSampling}
            disabled={micState !== "idle"}
            title="Thu bộ 100 câu giọng thật để đo nhận dạng (chỉ bản dev)"
            className="rounded-lg px-2 py-1.5 text-xs text-slate-600 hover:bg-pink-100 disabled:opacity-40"
          >
            {samples ? "Thoát thu mẫu" : "Thu mẫu"}
          </button>
        )}
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

      {samples && (
        <section aria-label="Thu mẫu giọng nói" className="mx-4 mb-2 rounded-xl border border-amber-200 bg-amber-50 p-3 text-sm">
          <p className="text-xs text-amber-700">
            Câu {Math.min(sampleIdx + 1, samples.length)}/{samples.length} · đã thu {samples.filter((x) => x.recorded).length} ·{" "}
            {currentSample?.category}
            {currentSample?.recorded && " · đã có (thu lại sẽ ghi đè)"}

          </p>
          <p className="my-2 text-base font-medium text-slate-800">
            {currentSample ? currentSample.text : "Xong hết rồi! Báo Claude chạy đo WER nhé."}
          </p>
          <p className="text-xs text-amber-700">
            Giữ F2 đọc câu trên, thả ra để lưu.
            {currentSample?.category === "nhieu_nhanh" && " Câu này: đọc nhanh hoặc bật quạt/nhạc nền."}
          </p>
          {lastSaved && (
            <div className="mt-2 rounded-lg bg-white/70 p-2 text-xs text-slate-600">
              <p>
                Vừa lưu {lastSaved.file} ({lastSaved.secs.toFixed(1)} s). Câu gốc: <span className="text-slate-800">{lastSaved.text}</span>
              </p>
              <p>
                Mori nghe được: <span className="font-medium text-slate-800">{lastSaved.heard || "(không nghe thấy gì)"}</span>
              </p>
              <p className="text-amber-700">Lệch nhiều (đọc nhầm hoặc nghe sai)? Bấm "← Câu trước" để thu lại.</p>
            </div>
          )}
          <div className="mt-2 flex gap-2">
            <button onClick={() => setSampleIdx((i) => Math.max(0, i - 1))} disabled={sampleIdx === 0 || micState !== "idle"} className="rounded-lg bg-white px-2 py-1 text-xs disabled:opacity-40">
              ← Câu trước
            </button>
            <button onClick={() => setSampleIdx((i) => Math.min(samples.length, i + 1))} disabled={sampleIdx >= samples.length || micState !== "idle"} className="rounded-lg bg-white px-2 py-1 text-xs disabled:opacity-40">
              Câu sau →
            </button>
          </div>
        </section>
      )}

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
          ref={draftRef}
          rows={2}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={
            micState === "listening"
              ? "Đang nghe… thả F2 hoặc bấm micro để dừng, Esc để hủy"
              : micState === "transcribing"
                ? "Đang nhận dạng giọng nói…"
                : ready
                  ? "Nhắn cho Mori… (Enter gửi, giữ F2 để nói)"
                  : "Đợi Mori sẵn sàng…"
          }
          className="flex-1 resize-none rounded-xl border border-pink-200 bg-white px-3 py-2 text-sm outline-none focus:border-pink-400"
        />
        <button
          type="button"
          onClick={() => (micState === "listening" ? stopMic() : startMic())}
          disabled={micState === "transcribing"}
          aria-pressed={micState === "listening"}
          aria-label={micState === "listening" ? "Dừng nghe" : "Nói (giữ F2)"}
          title={micState === "listening" ? "Dừng nghe" : "Nói (giữ F2)"}
          className={`rounded-xl px-3 py-2 text-sm disabled:opacity-40 ${
            micState === "listening" ? "animate-pulse bg-rose-500 text-white" : "bg-pink-100 text-pink-700 hover:bg-pink-200"
          }`}
        >
          {micState === "transcribing" ? "…" : "🎙"}
        </button>
        {generating || speaking ? (
          <button type="button" onClick={() => api.stopGeneration()} className="rounded-xl bg-slate-700 px-4 py-2 text-sm text-white hover:bg-slate-800">
            {generating ? "Dừng" : "Ngắt"}
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
