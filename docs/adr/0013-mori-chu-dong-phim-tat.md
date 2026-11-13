# ADR-0013: Mori chủ động bắt chuyện + phím gọi toàn cục Ctrl+Space+Q

**Trạng thái:** Chấp nhận
**Ngày:** 2026-11-13

## Bối cảnh
SPEC §10.1 (bản đầu): "Không tự bắt chuyện khi chưa được phép". Push-to-talk chỉ chạy khi cửa sổ Mori đang focus (F2) hoặc bấm nút 🎙.

Người dùng thử MVP (2026-11-13) thấy Mori "buồn tẻ": hoạt cảnh lặp, phải click vào nhân vật/cửa sổ mới nói được, "thiếu tính cá nhân hoá". Họ muốn Mori "như có nhân vật thật sống trong máy" và đã chọn:
- **Mori chủ động**: chào theo giờ khi mở máy (gọi tên, nhắc điều đã nhớ), nhắc nghỉ khi dùng máy lâu, bong bóng nói bâng quơ khi rảnh.
- **Phím gọi toàn cục `Ctrl + Space + Q`**: giữ = nói, thả = gửi; bấm nhanh = bật nghe, bấm lại = gửi.

## Quyết định
1. **Chủ động** (setting `proactive`, mặc định bật, checkbox "Mori chủ động" trên khung chat):
   - Chào: lần model sẵn sàng đầu tiên của phiên, chỉ khi lần chào trước cách ≥ 4 giờ (`last_greet`). Gọi model qua đúng `GenerationSlot` như lượt thường (người dùng nói trước thì nhường); ghi chú của app (giờ, buổi) **không lưu DB**; câu chào lưu vào hội thoại gần nhất như tin của Mori; đọc thành tiếng nếu bật "Mori nói".
   - Nhắc nghỉ: dùng máy liên tục 90 phút (rảnh ≥ 5 phút thì đếm lại) → một câu soạn sẵn, không gọi model. Thời gian rảnh lấy từ `GetLastInputInfo` — **chỉ là một con số mili giây**, không biết phím/chuột nào.
   - Bong bóng bâng quơ + hoạt cảnh theo giờ: chỉ ở cửa sổ avatar, câu soạn sẵn, chỉ chữ, không gọi model, không tiếng.
   - Không nói khi đang có lượt trả lời hoặc đang phát tiếng.
2. **Phím tắt**: Space không phải phím bổ trợ nên `RegisterHotKey` không đăng ký được tổ hợp này → low-level keyboard hook `WH_KEYBOARD_LL` trên thread riêng (`hotkey.rs`). Hook **chỉ xét** 3 mã phím Ctrl/Space/Q, giữ trạng thái bằng 3 cờ bool, **không lưu, không log, không gửi** phím nào; chỉ "nuốt" phím Q khi đang giữ Ctrl+Space; gửi sự kiện `hotkey-ptt` (true/false) tới khung chat. F2 trong cửa sổ Mori vẫn dùng được.

## Phương án đã cân nhắc
- `RegisterHotKey` với Ctrl+Alt+M — không cần hook, nhưng người dùng đã chọn Ctrl+Space+Q. Giữ làm phương án thay nếu họ thấy Ctrl+Space vướng (đổi bộ gõ, gợi ý code).
- Plugin `tauri-plugin-global-shortcut` — thêm gói, cũng dựa trên `RegisterHotKey` nên không nhận Space làm phím bổ trợ.
- Bắt chuyện bằng model cả cho bong bóng/nhắc nghỉ — tốn GPU/RAM lúc người dùng đang làm việc khác, chiếm slot llama-server; câu soạn sẵn đủ cho mục đích "có mặt".

## Hệ quả
- SPEC §10.1 đổi: Mori được chủ động chào/nhắc nghỉ/bong bóng, tắt được bằng "Mori chủ động". §9.2 thêm cảm xúc và hoạt cảnh theo giờ. §14.2 thêm dòng về hook bàn phím và `GetLastInputInfo`.
- Hệ quả đã báo người dùng: Ctrl+Space vẫn tới ứng dụng đang mở (chỉ Q bị nuốt) → có thể bật gợi ý code/đổi bộ gõ ở app khác.
- Audit Phase 7 (2026-11-13): 0 Critical/High/Medium; đã sửa 4 Low — hook đọc Ctrl/Space bằng `GetAsyncKeyState` lúc Q xuống (cờ tự theo dõi có thể kẹt khi nhả phím trên màn hình UAC/khóa máy → gõ "q" lại mở micro), tự kết thúc lượt khi mất sự kiện nhả Q, bỏ phím giả lập (`LLKHF_INJECTED`); lời chào chạy như việc nền (`BackgroundSlot`) → người dùng nói/gửi là hủy ngay, không bị chặn; không nói khi micro đang thu. Chấp nhận (người dùng muốn): lời chào có thể nhắc điều đã nhớ thành tiếng và hiện bong bóng — người ngồi cạnh/đang chia sẻ màn hình có thể thấy; tắt bằng "Mori chủ động".
- Hook toàn cục là bề mặt nhạy cảm (keylogger nếu bị sửa): phạm vi code nhỏ, có test `on_q`, phải qua audit; mọi thay đổi `hotkey.rs` cần giữ "chỉ xét phím Q (+ trạng thái Ctrl/Space), không lưu/log".
- Xem lại nếu: người dùng thấy chủ động gây phiền (giảm tần suất/thêm giờ im lặng), hoặc antivirus cảnh báo hook (chuyển Ctrl+Alt+M + `RegisterHotKey`).
