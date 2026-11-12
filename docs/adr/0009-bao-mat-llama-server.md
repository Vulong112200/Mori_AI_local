# ADR-0009: Gia cố bảo mật tiến trình llama-server

**Trạng thái:** Chấp nhận (2026-11-12)
**Ngày:** 2026-11-12

## Bối cảnh
Phase 1 chạy llama-server làm tiến trình con. Agent `security-auditor` rà mã Phase 1: 0 Critical, 0 High, 3 Medium, 5 Low. Các điểm chính:
- llama.cpp đọc biến môi trường `LLAMA_*` cho gần như mọi cờ, kể cả tools chạy shell, log file, tải model → môi trường có thể âm thầm phá luật "không chạy lệnh, không log nội dung, offline".
- Nếu Mori crash / bị End Task, llama-server (6 GB) mồ côi.
- API key trên command line đọc được qua `Win32_Process`.
- Thiếu `max_tokens`, cắt lịch sử chỉ theo số tin nhắn, hai lượt gửi song song làm hỏng nút Dừng, HTTP không timeout.

## Quyết định
- Xóa mọi biến `LLAMA_*` khỏi môi trường tiến trình con; key truyền qua `LLAMA_API_KEY` (env), sinh bằng `getrandom` (128 bit).
- Cờ cố định: `--host 127.0.0.1 --port <ngẫu nhiên> --no-webui --no-slots --offline`; stdout/stderr → null.
- Windows Job Object `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` bao tiến trình con.
- `max_tokens` 1024; lịch sử ≤ 24 tin và ≤ 12.000 ký tự; một lượt sinh tại một thời điểm (`GenerationSlot`); chỉ lưu tin nhắn khi model Ready.
- HTTP: `no_proxy`, connect timeout 2 s, `/health` timeout 2 s, warm-up 60 s.
- CSP thêm `base-uri 'none'; form-action 'none'; object-src 'none'`; log giữ 14 ngày; `personality.md` ≤ 8.000 ký tự.

Đã kiểm chứng trên máy: gọi không key → 401 (cả `/slots`); key không có trên command line; đặt `LLAMA_ARG_LOG_FILE` → file không được tạo; ép tắt `mori.exe` → llama-server chết theo.

## Phương án đã cân nhắc
- `env_clear()` toàn bộ — chặn chắc hơn nhưng llama.cpp/Vulkan driver cần `SystemRoot`, `PATH`… → chỉ xóa `LLAMA_*`.
- Theo dõi PID và kill khi khởi động lần sau — không xử lý được crash giữa phiên; Job Object do kernel đảm bảo.

## Hệ quả
- Phụ thuộc mới: `getrandom 0.3`, `windows-sys 0.61` (đã có sẵn gián tiếp trong Cargo.lock).
- **Để sau** (ROADMAP "Bảo mật — để sau"): ACL `E:\Mori Data` chỉ cho user hiện tại (cần người dùng đồng ý — đổi quyền thư mục); kiểm SHA-256 `llama-server.exe` + model theo manifest (Model Manager, Phase 6). Đã làm (2026-11-12): thu hẹp capability `core:default` → `core:event:allow-listen/unlisten`; loại trừ Windows Error Reporting (`WerAddExcludedApplication`, HKCU, theo tên exe — lưu vĩnh viễn, gỡ bằng `reg delete`); xóa dump Crashpad của WebView2 lúc khởi động (cờ `--disable-crash-reporter` không có tác dụng, đã thử). Rủi ro còn lại: upload dump WebView2 trước khi bị xóa; chưa thử crash thật để xác nhận WER không ghi LocalDumps.
- **Phase 2 (2026-11-12), whisper-server:** không có API key → route sau `--request-path` ngẫu nhiên 128 bit (test: GET/POST thiếu path → 404). Rủi ro chấp nhận: (1) khoảng hở giữa `free_port()` và lúc server bind (httplib dùng `SO_REUSEADDR`) — tiến trình cùng máy chiếm cổng trước có thể nhận audio; phải đoán đúng 1/~16k cổng, áp dụng cả llama-server; sửa triệt để = kiểm PID chủ socket (`GetExtendedTcpTable`) nếu cần. (2) Giữ kết nối treo (slowloris) tới cổng loopback → STT chậm tới timeout 60 s; chỉ ảnh hưởng tính sẵn sàng.
- **Phase 3 (2026-11-13), TTS worker Python:** script nhúng trong binary, `python -I` (bỏ PYTHON*/user site), `HF_HUB_OFFLINE=1` + `local_files_only` (audit: không nạp module mạng nào), `ZEROTTS_VOICES_HOME` trỏ vào data root, giọng chỉ nhận tên thư mục có sẵn (kiểm ở cả Rust và Python), stderr → null, `ort.disable_telemetry_events()` (ETW của ONNX Runtime). Output worker được kiểm (tần số 8–192 kHz, chunk ≤ 4 s), worker im lặng 30 s khi đang chờ → khởi động lại. Job Object thêm `DIE_ON_UNHANDLED_EXCEPTION` cho mọi worker → crash không gọi WER (trừ fail-fast — rủi ro còn lại).
