# Bàn giao cho session tiếp theo (2026-11-13)

> Đọc file này trước, rồi [CLAUDE.md](../CLAUDE.md) (luật bất biến), [ROADMAP.md](ROADMAP.md) (checkbox chi tiết), [ARCHITECTURE.md](ARCHITECTURE.md) (hệ thống đang chạy). Trao đổi với người dùng bằng tiếng Việt.

## 1. Người dùng & cách làm việc

- Người dùng bận, muốn Claude **tự làm trọn**: tự đo/kiểm, tự commit + push lên `origin/main` (repo vẫn commit thẳng `main`), cập nhật ROADMAP/ARCHITECTURE/MODEL_EVALUATION theo tiến độ. Chỉ hỏi khi cần quyết định của họ (tải gói/phần mềm mới, quyền admin/ACL, đổi quy tắc sản phẩm); gom câu hỏi vào một lần, có phương án khuyến nghị.
- Họ đánh giá Mori theo cảm giác **sống, tự nhiên, thông minh**. Đã từng chê thiết kế bắt nói câu lệnh + tự vào duyệt ghi nhớ là "phiền, dởm" → đổi sang Mori tự ghi nhớ (ADR-0012). Ưu tiên: tự động + có báo + Hoàn tác, thay vì bước thủ công.
- Kiểm chất lượng câu trả lời bằng **model thật** và **đọc output** (assert lỏng đã từng để lọt lỗi). Nói thẳng giới hạn, kèm phương án xấp xỉ.
- Chấm MVP chủ quan: **4/5**.

## 2. Tiến độ hiện tại

**Phase 0–5 đã qua gate.** Commit mới nhất trên `main`: xem `git log` (≥ `0fd773b`).

| Phase | Kết quả chính |
|---|---|
| 3 Giọng nói ra | e2e "thả F2 → nghe tiếng" P95 3.45–3.72 s (`cargo test e2e_gate -- --ignored`) |
| 4 Memory | Mori **tự ghi nhớ** sau mỗi lượt (ADR-0012), tóm tắt hội thoại dài, xóa có kiểm chứng, sao lưu/khôi phục; AI-06/AI-08 đạt (`cargo test llm_gate -- --ignored`) |
| 5 Avatar | Live2D Koharu (Cubism SDK for Web 5-r.5) cửa sổ trong suốt, nhép miệng, trạng thái nghe/nghĩ/nói, 3 chế độ; 30 FPS, TTFT +0.9 % |
| 6 Ổn định MVP | **gần xong** — xem mục 3 |

**Phase 6 đã làm:** WebView2 Fixed Version 157 + 145 luật firewall chặn outbound (`scripts/firewall.ps1`, admin) → **0 kết nối ra ngoài**; ACL `E:\Mori Data` và `D:\Mori_AI_Local` chỉ còn user + SYSTEM + Administrators; kiểm SHA-256 model/chương trình (`config/manifest.sha256`, `integrity.rs`; sai thì không chạy LLM); nút 🔌 Offline; idle 30 phút RAM không tăng; CPU fallback (ẩn GPU vẫn chạy); worker TTS chết tự dựng lại; mở lại loa khi đổi thiết bị (code, chưa thử thật); audit cuối 0 Critical/High.

## 3. Việc còn lại để chốt gate 6

1. **Hội thoại liên tục 30 phút** (SPEC §19.5): lần 1 dừng ở 71 lượt/~10 phút (RAM llama+whisper+TTS đứng yên 9.40 GB, TTFT TB 429 ms, e2e TB 2.96 s, max 3.81 s). Lần 2 (đủ 30 phút) đang chạy lúc viết file này — nếu chưa có số trong ROADMAP thì chạy lại khi **app tắt**:
   ```powershell
   cd src-tauri; $env:MORI_SOAK_MINUTES=30; cargo test e2e_gate -- --ignored --nocapture
   ```
   đồng thời đo RAM mỗi phút các tiến trình `llama-server`, `whisper-server`, `python` (Get-Process WorkingSet64). Đạt = không treo, RAM không tăng dần.
2. **Người dùng thử**: rút/cắm tai nghe trong lúc dùng (loa phải mở lại ở lượt sau — `player.rs`), sleep/resume máy rồi nói tiếp.
3. `/gate 6` → tick, cập nhật trạng thái, commit + push.

## 4. Yêu cầu mới của người dùng → Phase 7 (sống động & cá nhân hoá)

Nguyên văn (2026-11-13):
> "Hiện tại hoạt cảnh 2D chỉ có các cảnh lặp đi lặp lại, sao không tạo các cảnh ngẫu nhiên? Hơn nữa mỗi khi dùng đều phải click vào mô hình nhân vật hoặc cửa sổ app thì mới có thể nói được, sao không tạo một bộ phím gõ để truy cập gọi mori ngay? Các hoạt cảnh 2D linh động hơn là cách giúp tôi giải stress, hơn nữa tuân theo thời gian thực như sáng thì khi mở máy dậy thì Mori xuất hiện trong chăn đệm và từ từ dậy rồi gấp chăn đệm, có thể ngồi chơi một mình, làm việc, ngủ, nghe nhạc, học tập, ăn cơm, tắm rửa, thay quần áo, nhiều biểu cảm hơn, và AI hiện tại cũng thiếu tính cá nhân hoá quá, chỉ có miệng hoạt động được khi nói, cũng hơi buồn tẻ."

Người dùng đã chọn:
- **Phím tắt toàn cục `Ctrl + Space + Q`**: giữ = nói, thả = gửi; bấm nhanh = bật nghe, bấm lại = gửi. (Đã giải thích: phải dùng low-level keyboard hook; Ctrl+Space vẫn tới app đang mở; chỉ nuốt phím Q. Nếu họ thấy phiền → đổi Ctrl+Alt+M.)
- **Mori chủ động** (bỏ quy tắc SPEC §10.1 "không tự bắt chuyện" — cần **ADR-0013**): chào theo giờ khi mở máy (gọi tên, nhắc điều đã nhớ), nhắc nghỉ khi dùng máy lâu, bong bóng nói bâng quơ khi rảnh, **càng nhiều cảnh càng tốt — "như có nhân vật thật sống trong máy"**.
- **Cảnh cần hình vẽ mới** (chăn đệm, tắm, thay đồ): model mẫu Koharu **không có** → làm **bản xấp xỉ** (động tác + biểu cảm Koharu + đạo cụ vẽ chồng emoji/HTML + bong bóng chữ) **và** viết `docs/CHARACTER_SPEC.md` (đặc tả cho họa sĩ vẽ/rig nhân vật riêng — ADR-0006 vốn yêu cầu nhân vật riêng trước khi phát hành).

Kế hoạch chi tiết, checkbox: **ROADMAP.md → "Phase 7"**. Tóm tắt thứ tự đề xuất:
1. ADR-0013 + sửa SPEC §10.1, §9.2, §14.
2. Phím tắt: hoàn thiện + nối `hotkey.rs`.
3. Động tác ngẫu nhiên (mọi nhóm motion: Idle×3, Tap×4, Flick×4 — hiện chỉ dùng Idle).
4. Biểu cảm theo cảm xúc câu trả lời (`emotion.rs`, preset tham số Koharu).
5. Lịch sinh hoạt theo giờ máy + đạo cụ + bong bóng (AvatarView).
6. Chủ động: chào + nhắc nghỉ (`proactive.rs`), checkbox "Mori chủ động".
7. `docs/CHARACTER_SPEC.md`.
8. Kiểm: fmt/clippy/test/tsc, ảnh `avatar_snapshot` từng biểu cảm, FPS + TTFT A/B (MODEL_EVALUATION §5.2), audit (hook bàn phím, chủ động nói).

## 5. Code nháp đã có (CHƯA nối, CHƯA build)

Không có `mod` trong `lib.rs` nên không ảnh hưởng build hiện tại. Khi nối cần thêm feature `windows-sys`: `Win32_UI_WindowsAndMessaging`, `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_SystemInformation`.

| File | Nội dung | Cần làm để nối |
|---|---|---|
| `src-tauri/src/hotkey.rs` | Hook `WH_KEYBOARD_LL` (thread riêng + message loop), `on_key` thuần (có test), gửi `hotkey-ptt` true/false tới cửa sổ `main` qua channel | `mod hotkey;` + `hotkey::start(app.handle().clone())` trong setup; App.tsx nghe `hotkey-ptt`: down → `startMic()`; up → giữ < 400 ms thì chế độ bật/tắt (lần bấm sau mới `stopMic()`), ngược lại `stopMic()` |
| `src-tauri/src/emotion.rs` | `of(text)` → happy/love/sad/surprised/shy/worried/neutral (emoji + từ khóa, có test) | `mod emotion;`; trong `send_message` cộng dồn delta, khi nhãn đổi `emit_to("avatar","avatar-emotion",nhãn)` — **chỉ nhãn, không nội dung** |
| `src-tauri/src/proactive.rs` | `greet()` (lần Ready đầu phiên, cách lần trước ≥ 4 h, qua `GenerationSlot`, ghi chú app không lưu DB, lưu câu chào vào hội thoại gần nhất, emit `proactive-message` + `avatar-say`/`avatar-emotion`, đọc nếu bật "Mori nói"), `start_break_reminder()` (GetLastInputInfo: 90 phút liên tục, rảnh ≥ 5 phút đếm lại), `local_time()`, `part_of_day()`, setting `proactive` (mặc định bật) | `mod proactive;`; `GenerationSlot`, `context`, `prefill_next` dùng được từ module con; gọi `greet` trong callback Ready, `start_break_reminder` trong setup; bỏ alias `Stop` thừa cuối file; UI: checkbox "Mori chủ động", App nghe `proactive-message`, AvatarView nghe `avatar-say` (bong bóng 6–8 s) |

Tham số Koharu (Cubism 3, tên `PARAM_*`) dùng cho biểu cảm/đạo cụ: `PARAM_EYE_L/R_SMILE`, `PARAM_TEAR_L/R`, `PARAM_BROW_L/R_Y`, `PARAM_BROW_L/R_FORM`, `PARAM_MOUTH_FORM(_02)`, `PARAM_CHEEK`, `PARAM_DROOL`, `PARAM_EYE_01` (lấp lánh), `PARAM_EYE_SIZE`, `PARAM_HAND_SWITCH_L/R`, `PARAM_PONPON`. **Chưa biết giá trị/hình của `HAND_SWITCH`/`PONPON`** → bản dev có `avatar_snapshot` (lưu canvas vào `E:\Mori Data\logs\avatar_snapshot.png`, đọc được bằng Read) để chụp thử; min/max đọc bằng `getModel().getParameterMinimumValue/MaximumValue(index)`.

## 6. Lưu ý kỹ thuật (đã vấp)

- `npm run tauri dev` tự build lại + khởi động lại app khi sửa Rust → đừng sửa code khi đang đo bằng app; dừng app trước khi chạy test model thật (2 bộ 8B cùng lúc làm sai số đo / có lần mất kết nối). Vite cũ giữ cổng 1420 → tắt tiến trình `node … vite.js`.
- Mã Live2D (`src/live2d/framework`, `public/live2d`, `public/avatar`) **gitignored**, sinh bằng `node scripts/setup_avatar.mjs`; Framework được thêm `// @ts-nocheck` (mã bên thứ ba). Mã Live2D chỉ import động trong cửa sổ avatar (Core là biến toàn cục).
- Sửa file trên Windows: heredoc Bash hay làm hỏng `\` và `\n` trong Python → viết script ra file (Write) rồi chạy; `cargo fmt` đổi định dạng → mẫu thay thế cũ không còn khớp.
- `npx` luôn dùng `--no-install` (một lần chạy nhầm thư mục đã tự tải gói `tsc` rác).
- PowerShell 5.1 đọc `.ps1` UTF-8 không BOM sai dấu tiếng Việt → tên luật firewall dùng ASCII, file lưu có BOM.
- Đổi model/runtime → cập nhật `config/manifest.sha256`; đổi bản WebView2 Fixed → sửa hằng `WEBVIEW2_FIXED` (lib.rs) + manifest + chạy lại `scripts/firewall.ps1` (admin, UAC).
- Agent `security-auditor` có lần chạy 77 phút khi máy bận chạy model → giao việc hẹp, dặn "chỉ đọc code, không build/chạy model".
