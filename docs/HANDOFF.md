# Bàn giao cho session tiếp theo (2026-11-13, cuối ngày)

> Đọc file này trước, rồi [CLAUDE.md](../CLAUDE.md) (luật bất biến), [ROADMAP.md](ROADMAP.md) (checkbox chi tiết), [ARCHITECTURE.md](ARCHITECTURE.md) (hệ thống đang chạy). Trao đổi với người dùng bằng tiếng Việt.

## 1. Người dùng & cách làm việc

- Người dùng bận, muốn Claude **tự làm trọn**: tự đo/kiểm, tự commit + push lên `origin/main` (repo vẫn commit thẳng `main`), cập nhật ROADMAP/ARCHITECTURE/MODEL_EVALUATION theo tiến độ. Chỉ hỏi khi cần quyết định của họ (tải gói/phần mềm mới, quyền admin/ACL, đổi quy tắc sản phẩm); gom câu hỏi vào một lần, có phương án khuyến nghị.
- Họ đánh giá Mori theo cảm giác **sống, tự nhiên, thông minh**. Đã từng chê thiết kế bắt nói câu lệnh + tự vào duyệt ghi nhớ là "phiền, dởm" → đổi sang Mori tự ghi nhớ (ADR-0012). Ưu tiên: tự động + có báo + Hoàn tác, thay vì bước thủ công.
- Kiểm chất lượng câu trả lời bằng **model thật** và **đọc output** (assert lỏng đã từng để lọt lỗi). Nói thẳng giới hạn, kèm phương án xấp xỉ.
- Chấm MVP chủ quan: **4/5**.

## 2. Tiến độ hiện tại

**Phase 0–6 đã qua gate** (gate 6 chốt 2026-11-13; 2 mục thử tay dời sang gate 7). **Phase 7 đã code xong, chờ người dùng thử.**

| Phase | Kết quả chính |
|---|---|
| 3 Giọng nói ra | e2e "thả F2 → nghe tiếng" P95 3.1–3.7 s (`cargo test e2e_gate -- --ignored`) |
| 4 Memory | Mori tự ghi nhớ (ADR-0012), tóm tắt hội thoại dài, xóa có kiểm chứng, sao lưu/khôi phục |
| 5 Avatar | Live2D Koharu, cửa sổ trong suốt, nhép miệng, 3 chế độ |
| 6 Ổn định MVP | WebView2 Fixed + firewall (0 kết nối ra ngoài), ACL, SHA-256, idle/hội thoại 30 phút ổn định, CPU fallback |
| 7 Sống động | Xem mục 3 |

## 3. Phase 7 — đã làm (2026-11-13)

Yêu cầu nguyên văn của người dùng: xem ROADMAP "Phase 7". Đã làm (chi tiết + số đo: ROADMAP Phase 7, ADR-0013, ADR-0014, MODEL_EVALUATION §5.4):
- **Phím Ctrl+Space+Q** toàn cục (`hotkey.rs`): giữ = nói, bấm nhanh = bật nghe / bấm lại = gửi. Chỉ xét phím Q, Ctrl/Space đọc bằng `GetAsyncKeyState`, bỏ phím giả lập.
- **Biểu cảm theo cảm xúc** (`emotion.rs` → `avatar-emotion`), **mọi nhóm motion** Koharu, động tác ngẫu nhiên khi rảnh.
- **Hoạt cảnh theo giờ máy** (`AvatarView.tsx`, `avatar.ts`): ngủ, thức dậy, ăn, làm việc, học, nhạc, chơi pompom, buồn ngủ + đạo cụ emoji + bong bóng chữ. Chụp từng cảnh: `VITE_AVATAR_DEMO=1 npm run tauri dev` → `E:\Mori Data\logsvatar_snapshot_<tên>.png`.
- **Mori chủ động** (`proactive.rs`): chào khi mở máy (≥ 4 h từ lần trước, chạy như việc nền), nhắc nghỉ sau 90 phút; checkbox "Mori chủ động". Đã chào thật lúc test (setting `last_greet` đã ghi → lần chào kế tiếp sau 4 h).
- **Giờ máy trong prompt** (chỉ prompt, không DB) + personality (≤ 1 emoji, nhắc điều đã nhớ). `llm_gate` đạt.
- **Tối ưu RAM** (ADR-0014): llama-server `-fa on -ctk q8_0 -ctv q8_0 --cache-ram 1024` → 6319 → 5793 MB, TTFT không đổi. Đã thử, không dùng: tắt flash-attn, `-lm none`, STT beam 3/2 (WER tăng — người dùng chọn không tăng WER), WebView2 `--renderer-process-limit=1`.
- `docs/CHARACTER_SPEC.md` (đặc tả nhân vật riêng cho họa sĩ).
- Audit Phase 7: 0 Critical/High/Medium, 4 Low đã sửa.

## 4. Việc tiếp theo

1. **Người dùng thử** (gate 7): Ctrl+Space+Q từ app khác (giữ/bấm nhanh), biểu cảm khi trò chuyện, hoạt cảnh theo giờ, lời chào/nhắc nghỉ; rút/cắm tai nghe khi Mori đang nói; sleep/resume rồi nói tiếp.
2. Lúc họ thử: đo log `avatar_fps` khi hoạt động (phải 30) và TTFT (`llm_ttft_ms`) — chưa đo lại sau Phase 7.
3. Theo phản hồi: nếu Ctrl+Space vướng (đổi bộ gõ) → Ctrl+Alt+M + `RegisterHotKey` (ADR-0013); nếu chủ động phiền → giảm tần suất/giờ im lặng.
4. Đã ghi nhận chưa sửa: gần như câu nào Mori cũng kết bằng 😊 (sau khi cho phép 1 emoji) → biểu cảm hay "happy"; 1 lần Mori xưng "mình" và nói "bạn đã nói trước đó" sai (`llm_gate`). Nếu người dùng thấy lặp → bớt emoji trong personality.
5. `/gate 7` khi người dùng xác nhận.

## 5. Tham số Koharu (cho biểu cảm/đạo cụ)

`PARAM_EYE_L/R_SMILE`, `PARAM_TEAR_L/R`, `PARAM_BROW_L/R_Y/FORM`, `PARAM_MOUTH_FORM(_02)`, `PARAM_CHEEK`, `PARAM_EYE_01` (lấp lánh), `PARAM_EYE_SIZE`, `PARAM_HAND_SWITCH_L/R` (2 = pompom, 3 = đồ tay trái), `PARAM_PONPON`. Motion: 01 vẫy tay vui, 02/03 trầm ngâm, 04 cười híp mắt, 05 khóc, 06 đứng, 07/08 pompom, 09 cầm đồ, idle/idle_02. Preset trong `avatar.ts` dùng giá trị chuẩn hóa −1…1 theo min/max đọc từ model.

## 6. Lưu ý kỹ thuật (đã vấp)

- `npm run tauri dev` tự build lại + khởi động lại app khi sửa Rust → đừng sửa code khi đang đo bằng app; dừng app trước khi chạy test model thật (2 bộ 8B cùng lúc làm sai số đo / có lần mất kết nối). Vite cũ giữ cổng 1420 → tắt tiến trình `node … vite.js`.
- Mã Live2D (`src/live2d/framework`, `public/live2d`, `public/avatar`) **gitignored**, sinh bằng `node scripts/setup_avatar.mjs`; Framework được thêm `// @ts-nocheck` (mã bên thứ ba). Mã Live2D chỉ import động trong cửa sổ avatar (Core là biến toàn cục).
- Sửa file trên Windows: heredoc Bash hay làm hỏng `\` và `\n` trong Python → viết script ra file (Write) rồi chạy; `cargo fmt` đổi định dạng → mẫu thay thế cũ không còn khớp.
- `npx` luôn dùng `--no-install` (một lần chạy nhầm thư mục đã tự tải gói `tsc` rác).
- PowerShell 5.1 đọc `.ps1` UTF-8 không BOM sai dấu tiếng Việt → tên luật firewall dùng ASCII, file lưu có BOM.
- Đổi model/runtime → cập nhật `config/manifest.sha256`; đổi bản WebView2 Fixed → sửa hằng `WEBVIEW2_FIXED` (lib.rs) + manifest + chạy lại `scripts/firewall.ps1` (admin, UAC).
- Agent `security-auditor` có lần chạy 77 phút khi máy bận chạy model → giao việc hẹp, dặn "chỉ đọc code, không build/chạy model".
