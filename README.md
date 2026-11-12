# Mori Desktop Companion

Trợ lý AI desktop chạy 100% local trên Windows 11 — hội thoại tiếng Việt bằng giọng nói, memory do người dùng kiểm soát, avatar Live2D.

**Trạng thái:** Phase 0 hoàn thành (đã chọn model: Qwen3-8B, Whisper turbo, ZeroTTS int8, Live2D). Phase 1 — app chat văn bản chạy được (`npm run tauri dev`), chờ thử để qua gate.

## Tài liệu

- [docs/ROADMAP.md](docs/ROADMAP.md) — tiến độ, việc kế tiếp (mục Bàn giao ở đầu)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — kiến trúc thực tế, cách chạy
- [docs/SPEC.md](docs/SPEC.md) — đặc tả sản phẩm & kỹ thuật
- [docs/SPEC_REVIEW.md](docs/SPEC_REVIEW.md) — rà soát bản spec v0
- [docs/adr/](docs/adr/) — quyết định kiến trúc
- [docs/MODEL_EVALUATION.md](docs/MODEL_EVALUATION.md) · [docs/HARDWARE_REPORT.md](docs/HARDWARE_REPORT.md) — số đo Phase 0

## Làm việc với Claude Code

Quy tắc trong [CLAUDE.md](CLAUDE.md). Lệnh dự án:

| Lệnh | Việc |
|---|---|
| `/status` | Tóm tắt tiến độ, blocker, việc kế tiếp |
| `/adr <tiêu đề>` | Tạo ADR mới |
| `/bench <llm\|stt\|tts>` | Chạy benchmark Phase 0, ghi kết quả |
| `/gate <n>` | Kiểm tra gate của phase n |

Model, runtime, DB nằm ở `E:\Mori Data\` — không có trong repo.
