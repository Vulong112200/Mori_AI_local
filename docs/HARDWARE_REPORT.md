# Báo cáo phần cứng

> Điền từ output của `powershell -ExecutionPolicy Bypass -File scripts/phase0/hw_inventory.ps1`. Chỉ ghi số đo được trên máy đích.

**Ngày đo:** 2026-11-11 22:48 (lần 1, chạy pin 86%, power plan Balanced)

## 1. Tổng quan

| Mục | Giá trị |
|---|---|
| CPU | Intel Core Ultra 9 285H — 16 nhân / 16 luồng (không có HT) |
| RAM tổng / trống | 31.4 GB / **8.6 GB** (lúc đo) |
| GPU + driver | Intel Arc 140T (Windows báo 16 GB dùng chung) — 32.0.101.8860 |
| NPU + driver | Intel AI Boost — 32.0.100.4023 |
| OS build | Windows 11 Home Single Language, build 26300 |
| Pagefile | **Windows tự quản lý**, đang cấp 2.048 MB (dùng 134 MB, đỉnh 200 MB) |
| Commit limit / trống | 33.4 GB / **4.2 GB** (lúc đo) |
| Ổ E: còn trống | 155 GB / 265 GB |
| `E:\Mori Data` | Chưa tạo |
| Toolchain | Git, Node, Cargo/Rustc, Python 3.14 có; **uv, CMake thiếu** |

### Nhận xét lần đo 1

- Spec v0 ghi pagefile "cố định 2 GB" là **sai**: pagefile do Windows quản lý, hiện chỉ cấp 2 GB và sẽ tự nới ra khi cần.
- Lúc đo, hệ thống đã dùng ~29 GB commit, chỉ còn **4.2 GB** — chưa đủ cho LLM 4B (3–5 GB) + STT + TTS. Trước khi benchmark cần biết tiến trình nào đang chiếm RAM (Task Manager → Memory) và đóng bớt. Đo lại RAM trống ở trạng thái "làm việc bình thường" để ra ngân sách thực.

## 2. Thiết bị audio

| Loại | Tên thiết bị |
|---|---|
| Micro | Microphone Array (Intel Smart Sound Technology for Digital Microphones) |
| Loa | Speakers (Realtek Audio) |
| Tai nghe | Headphones (Realtek Audio) |

## 3. GPU backend cho llama.cpp

| Backend | Nhận GPU? | Lỗi | Ghi chú |
|---|---|---|---|
| Vulkan | _chưa thử_ | | `llama-cli --list-devices` |
| SYCL | _chưa thử_ | | |

## 4. Nhiệt & ổn định

| Test | Kết quả |
|---|---|
| LLM sinh liên tục 10 phút: tg đầu → cuối (throttle?) | _chưa đo_ |
| Cắm sạc vs pin (chênh lệch tg) | _chưa đo_ |

## 5. Ghi chú / vấn đề

- Benchmark nên chạy khi **cắm sạc** và power plan "Best performance"; ghi điều kiện vào mỗi dòng kết quả.
