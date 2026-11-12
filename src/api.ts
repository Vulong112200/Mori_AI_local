// Lớp gọi lệnh Rust. Frontend không nói chuyện trực tiếp với model — chỉ qua các lệnh này.
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Role = "user" | "assistant";

export interface Message {
  id: string;
  role: Role;
  content: string;
  created_at: string;
}

export type LlmStatus =
  | { state: "starting"; model: string }
  | { state: "ready"; model: string }
  | { state: "restarting"; model: string; attempt: number }
  | { state: "error"; message: string }
  | { state: "stopped" };

export type ChatEvent =
  | { type: "delta"; text: string }
  | { type: "done"; message: Message | null; stopped: boolean };

export interface InitState {
  conversation_id: string;
  messages: Message[];
  status: LlmStatus;
  fast_mode: boolean;
  mic: string | null;
  voice_out: boolean;
  tts_voice: string;
  tts_voices: string[];
}

export interface Mic {
  id: string;
  name: string;
}

export const init = () => invoke<InitState>("init");
export const newConversation = () => invoke<string>("new_conversation");
export const stopGeneration = () => invoke<void>("stop_generation");
export const setFastMode = (enabled: boolean) => invoke<void>("set_fast_mode", { enabled });
export const listMics = () => invoke<Mic[]>("list_mics");
export const setMic = (id: string | null) => invoke<void>("set_mic", { id });
// Push-to-talk: audio nằm trong RAM phía Rust, frontend chỉ nhận transcript.
export const startListening = () => invoke<void>("start_listening");
export const stopListening = () => invoke<string>("stop_listening");
export const cancelListening = () => invoke<void>("cancel_listening");

// Thu bộ câu giọng thật cho gate Phase 2 — chỉ bản dev.
export interface Sample {
  file: string;
  category: string;
  text: string;
  recorded: boolean;
}
export const listSamples = () => invoke<Sample[]>("list_samples");
export const saveSample = (file: string) => invoke<{ secs: number; heard: string }>("save_sample", { file });

export function sendMessage(conversationId: string, text: string, onEvent: (e: ChatEvent) => void) {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = onEvent;
  return invoke<Message>("send_message", { conversationId, text, onEvent: channel });
}

export const setVoiceOut = (enabled: boolean) => invoke<void>("set_voice_out", { enabled });
export const setTtsVoice = (voice: string) => invoke<void>("set_tts_voice", { voice });
export const onTtsPlaying = (cb: (playing: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("tts-playing", (e) => cb(e.payload));
export const onTtsStatus = (cb: (s: "loading" | "ready" | "error") => void): Promise<UnlistenFn> =>
  listen<"loading" | "ready" | "error">("tts-status", (e) => cb(e.payload));

export const onLlmStatus = (cb: (s: LlmStatus) => void): Promise<UnlistenFn> =>
  listen<LlmStatus>("llm-status", (e) => cb(e.payload));
