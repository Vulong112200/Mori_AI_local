# ADR-0001: Kiến trúc local-first Tauri + Rust + worker loopback

**Trạng thái:** Chấp nhận
**Ngày:** 2026-11-11

## Bối cảnh
Mori phải chạy AI 100% local trên Windows 11 laptop (Core Ultra 9 285H, 32 GB RAM, iGPU Arc 140T), có UI desktop nổi với avatar Live2D, và cho phép thay từng thành phần AI.

## Quyết định
- Tauri 2 + React/TS cho UI; Rust làm lõi, sở hữu mọi process con.
- LLM: `llama-server` (llama.cpp), STT: `whisper-server` (whisper.cpp) — giao tiếp HTTP, bind `127.0.0.1`, cổng ngẫu nhiên.
- TTS: worker riêng giao tiếp JSON Lines qua stdio (Python 3.12 qua uv nếu engine yêu cầu).
- Rust thu (cpal) và phát (rodio) audio; gửi biên độ RMS cho avatar qua event.
- SQLite + FTS5 cho dữ liệu.

## Phương án đã cân nhắc
- Electron — nặng RAM hơn, không có lợi thế cho bài toán này.
- Frontend tự phát audio bằng Web Audio — lip-sync dễ hơn, nhưng giọng nói phụ thuộc WebView; vi phạm nguyên tắc "AI độc lập avatar".
- JSON Lines cho mọi worker — llama/whisper đã có HTTP server sẵn, viết thêm lớp bọc là thừa.
- Nhúng llama.cpp vào Rust qua FFI — crash của model kéo sập app; process riêng an toàn hơn.

## Hệ quả
- Worker crash không làm sập UI; Rust cần health check + restart.
- Thay model/runtime chỉ cần đổi binary/manifest.
- Cần quản lý cổng và vòng đời process cẩn thận (SPEC §13).
