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
  avatar_mode: AvatarMode;
  avatar_click_through: boolean;
}

export interface Mic {
  id: string;
  name: string;
}

export const init = () => invoke<InitState>("init");
export interface Check {
  name: string;
  ok: boolean;
  detail: string;
}
export const readiness = () => invoke<Check[]>("readiness");
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

// Memory (SPEC §11, ADR-0012): Mori tự ghi nhớ sau mỗi lượt (+ lệnh "nhớ là …", nút ⭐); người dùng hoàn tác/sửa/xóa.
export type MemoryCategory = "so_thich" | "du_an" | "ca_nhan" | "khac";
export const MEMORY_CATEGORIES: Record<MemoryCategory, string> = {
  so_thich: "Sở thích",
  du_an: "Dự án",
  ca_nhan: "Cá nhân",
  khac: "Khác",
};
export const MAX_MEMORY_CHARS = 500;
export interface Memory {
  id: string;
  category: MemoryCategory;
  content: string;
  status: "proposed" | "approved";
  created_at: string;
}
export const listMemories = () => invoke<Memory[]>("list_memories");
export const onMemorySaved = (cb: (m: Memory) => void): Promise<UnlistenFn> =>
  listen<Memory>("memory-saved", (e) => cb(e.payload));
export const starMessage = (messageId: string) => invoke<Memory>("star_message", { messageId });
export const updateMemory = (id: string, content: string, category: MemoryCategory, approved: boolean) =>
  invoke<void>("update_memory", { id, content, category, approved });
export const deleteMemory = (id: string) => invoke<void>("delete_memory", { id });
export const deleteAllMemories = () => invoke<void>("delete_all_memories");
export const backupDb = () => invoke<string>("backup_db");
export const listBackups = () => invoke<string[]>("list_backups");
/** Trả tên bản sao lưu tự tạo ngay trước khi khôi phục. */
export const restoreBackup = (name: string) => invoke<string>("restore_backup", { name });

// Avatar (Phase 5). Trạng thái do Rust phát: nghe / nghĩ / rảnh; "nói" lấy từ tts-playing.
export type AvatarMode = "companion" | "chat" | "focus";
export const onAvatarState = (cb: (s: "idle" | "listening" | "thinking") => void): Promise<UnlistenFn> =>
  listen<"idle" | "listening" | "thinking">("avatar-state", (e) => cb(e.payload));
export const onTtsLevel = (cb: (level: number) => void): Promise<UnlistenFn> =>
  listen<number>("tts-level", (e) => cb(e.payload));
export const onAvatarMode = (cb: (m: AvatarMode) => void): Promise<UnlistenFn> =>
  listen<AvatarMode>("avatar-mode", (e) => cb(e.payload));
export const onAvatarPtt = (cb: (down: boolean) => void): Promise<UnlistenFn> =>
  listen<boolean>("avatar-ptt", (e) => cb(e.payload));
export const avatarDrag = () => invoke<void>("avatar_drag");
export const avatarPtt = (down: boolean) => invoke<void>("avatar_ptt", { down });
export const setAvatarMode = (mode: AvatarMode) => invoke<void>("set_avatar_mode", { mode });
export const setAvatarClickThrough = (enabled: boolean) => invoke<void>("set_avatar_click_through", { enabled });
export const avatarReport = (r: { fps?: number; active?: boolean; error?: string }) =>
  invoke<void>("avatar_report", r).catch(() => {});
export const avatarSnapshot = (png: string) => invoke<void>("avatar_snapshot", { png });
