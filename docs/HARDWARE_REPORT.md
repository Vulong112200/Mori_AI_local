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

### Lần đo 2 (2026-11-11 23:00)

RAM trống 7.8 GB, commit trống 3.0 GB, giới hạn commit đã tự nới lên 34.4 GB (pagefile Windows quản lý). Không có tiến trình nào chiếm bất thường: 24.7 GB private là tổng của nhiều tiến trình nhỏ (Chrome, Edge, VS Code, Claude…). Kết luận: không cần đổi pagefile; llama-server (4–5 GB) chạy được. Khi benchmark nên đóng bớt trình duyệt để số đo ít nhiễu.

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
| Vulkan (b12284) | ✅ `Vulkan0: Intel Arc 140T (18307 MiB, 17538 MiB free)` | Không | pp512 ~888 t/s, tg128 ~23 t/s với Qwen3-4B Q4_K_M, ổn định. Lượt đầu cần biên dịch shader (~4 s) |
| SYCL | Không thử | | Không cần: Vulkan đã đạt mục tiêu TTFT. Thử lại chỉ khi Vulkan gặp lỗi |
| whisper.cpp Vulkan (v1.9.5 tự build) | ✅ | Không | turbo q5_0: 1.53 s TB cho câu ngắn (CPU: 10.3 s) |
| OpenVINO | Chưa thử | | llama.cpp đã có bản dựng sẵn `win-openvino` (b12284) — hướng dự phòng / NPU sau MVP |

## 4. Nhiệt & ổn định

| Test | Kết quả |
|---|---|
| LLM sinh liên tục 10 phút: tg đầu → cuối (throttle?) | _chưa đo_ |
| Cắm sạc vs pin (chênh lệch tg) | _chưa đo_ |

## 5. Ghi chú / vấn đề

- Benchmark nên chạy khi **cắm sạc** và power plan "Best performance"; ghi điều kiện vào mỗi dòng kết quả.
