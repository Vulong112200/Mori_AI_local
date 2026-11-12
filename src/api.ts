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
}

export const init = () => invoke<InitState>("init");
export const newConversation = () => invoke<string>("new_conversation");
export const stopGeneration = () => invoke<void>("stop_generation");
export const setFastMode = (enabled: boolean) => invoke<void>("set_fast_mode", { enabled });

export function sendMessage(conversationId: string, text: string, onEvent: (e: ChatEvent) => void) {
  const channel = new Channel<ChatEvent>();
  channel.onmessage = onEvent;
  return invoke<Message>("send_message", { conversationId, text, onEvent: channel });
}

export const onLlmStatus = (cb: (s: LlmStatus) => void): Promise<UnlistenFn> =>
  listen<LlmStatus>("llm-status", (e) => cb(e.payload));
