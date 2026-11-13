# Mori Desktop Companion

Trợ lý AI desktop chạy 100% local trên Windows 11 — cô bạn đồng hành nói tiếng Việt bằng giọng nói hoặc văn bản, tự ghi nhớ (có Hoàn tác), nhân vật Live2D sống động theo giờ thật. Không cloud, không telemetry, 0 kết nối ra ngoài.

**Trạng thái (2026-11-13):** Phase 0–6 qua gate (MVP ổn định). Phase 7 (sống động & cá nhân hoá: phím Ctrl+Space+Q, biểu cảm theo cảm xúc, hoạt cảnh theo giờ, Mori tự chào/nhắc nghỉ, tối ưu RAM) đã code và **đã cài thành app** (tự chạy khi đăng nhập), chờ người dùng thử để qua gate 7.

| | Đang dùng |
|---|---|
| Não | Qwen3-8B Q4_K_M (chế độ nhanh: Qwen3-4B-Instruct-2507), llama.cpp Vulkan |
| Tai | Whisper large-v3 q5_0, whisper.cpp Vulkan (WER 9.3 % trên giọng thật) |
| Giọng | ZeroTTS int8 (giọng `maichi`) |
| Hình | Live2D Cubism Web, mẫu Koharu (dev) |
| Độ trễ | "thả phím → nghe tiếng" P95 3.1–3.7 s |

## Chạy

```powershell
npm run tauri dev                                                   # bản dev
npm run tauri build -- --no-bundle                                  # build bản release
powershell -ExecutionPolicy Bypass -File scripts\install.ps1         # cài: Start Menu + tự chạy khi đăng nhập
```

Chi tiết: [docs/ARCHITECTURE.md §8](docs/ARCHITECTURE.md). Model, runtime, DB nằm ở `E:\Mori Data\` — không có trong repo.

## Tài liệu

- [docs/HANDOFF.md](docs/HANDOFF.md) — bàn giao mới nhất: tiến độ, yêu cầu người dùng, việc tiếp theo
- [docs/ROADMAP.md](docs/ROADMAP.md) — tiến độ chi tiết theo phase
- [docs/AI_SYSTEM.md](docs/AI_SYSTEM.md) — cấu trúc & logic AI hiện tại (một lượt hội thoại, prompt, trí nhớ, chủ động)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — kiến trúc thực tế, mã nguồn, lệnh IPC, cách chạy/cài
- [docs/SPEC.md](docs/SPEC.md) — đặc tả sản phẩm & kỹ thuật · [docs/SPEC_REVIEW.md](docs/SPEC_REVIEW.md) — rà soát bản spec v0
- [docs/adr/](docs/adr/) — quyết định kiến trúc (0001–0014)
- [docs/MODEL_EVALUATION.md](docs/MODEL_EVALUATION.md) · [docs/HARDWARE_REPORT.md](docs/HARDWARE_REPORT.md) — số đo
- [docs/CHARACTER_SPEC.md](docs/CHARACTER_SPEC.md) — đặc tả nhân vật riêng cho họa sĩ

## Làm việc với Claude Code

Quy tắc trong [CLAUDE.md](CLAUDE.md). Lệnh dự án:

| Lệnh | Việc |
|---|---|
| `/status` | Tóm tắt tiến độ, blocker, việc kế tiếp |
| `/adr <tiêu đề>` | Tạo ADR mới |
| `/bench <llm\|stt\|tts>` | Chạy benchmark, ghi kết quả |
| `/gate <n>` | Kiểm tra gate của phase n |
