# ADR-0002: Data root E:\Mori Data, tách khỏi E:\Memora Data

**Trạng thái:** Chấp nhận
**Ngày:** 2026-11-11

## Bối cảnh
Spec v0 ghi lẫn `E:\Memora Data\` và `E:\Mori Data\`. Kiểm tra máy: `E:\Memora Data` là dữ liệu của một dự án khác đang tồn tại (có `models`, `runtime\whisper.cpp\b5454`, `database`...).

## Quyết định
- Toàn bộ dữ liệu Mori nằm ở `E:\Mori Data\` (cấu trúc SPEC §16).
- Mori **không ghi** vào `E:\Memora Data\` (chặn bằng `deny` trong `.claude/settings.json`).
- Phase 0 được **đọc/chạy** `E:\Memora Data\runtime\whisper.cpp\b5454\whisper-cli.exe` để benchmark, tránh tải trùng.

## Phương án đã cân nhắc
- Dùng chung `E:\Memora Data` — rủi ro ghi đè/xóa nhầm dữ liệu dự án kia, khó dọn dẹp riêng.

## Hệ quả
- Model có thể bị tải trùng giữa hai dự án (chấp nhận được, ổ E còn ~154 GB).
- Đường dẫn data root nên là setting cấu hình được, mặc định `E:\Mori Data`.
