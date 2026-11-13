# ADR-0012: Mori tự ghi nhớ, không cần duyệt

**Trạng thái:** Chấp nhận
**Ngày:** 2026-11-13

## Bối cảnh
SPEC §11.4 (bản đầu): AI chỉ **đề xuất** memory, người dùng duyệt thì mới vào prompt. Phase 4 làm theo hướng người dùng chọn lúc đầu: lệnh "Mori, nhớ là …" + nút ⭐ → đề xuất → panel duyệt.

Người dùng thử trong app (2026-11-13) và đánh giá rất kém: "lúc nào cũng phải nói mori à, nhớ là … rất phiền, còn phải tự vô duyệt". Lần thử đó còn thất bại hẳn vì STT chép "Mori ơi" thành "Morrie ơi" nên lệnh không được nhận ra; model lại nói dối "đã ghi nhớ". Người dùng chọn: **Mori tự nhớ, có báo kèm nút Hoàn tác, không cần duyệt**.

Đo (`cargo test llm_gate -- --ignored`, Qwen3-8B): llama-server b12284 lưu cache prompt trong RAM → một request nền khác prompt không phá cache hội thoại (lượt sau chỉ xử lý 10 token, 0.6–0.85 s, `scratchpad slot_test`), nên gọi model thêm sau mỗi lượt không làm chậm hội thoại.

## Quyết định
- Sau mỗi câu trả lời, **trong nền** (`after_reply` → `auto_remember`): model đọc câu người dùng vừa nói (+ câu Mori nói ngay trước, + những gì đã nhớ để khỏi lặp) và trả tối đa 3 dòng `- [nhóm] Người dùng …`, hoặc `KHÔNG`. Mỗi dòng lưu thẳng `status='approved'`, `source_message_id` = tin nhắn đó. Dòng trùng nguyên văn với ghi nhớ cũ thì bỏ.
- Bỏ qua: câu hỏi (kết thúc `?`), câu < 8 ký tự, đoạn > 1.500 ký tự (tài liệu dán vào, không phải lời kể về bản thân — giảm prompt injection qua tài liệu).
- Prompt trích: chỉ ghi điều người dùng **nói rõ**, không suy diễn; sửa lỗi nhận dạng giọng nói hiển nhiên; nội dung là dữ liệu.
- Lệnh "nhớ là …" (nhận cả tên bị STT chép sai) và nút ⭐ cũng lưu thẳng, không qua bước trích.
- UI: mỗi ghi nhớ mới hiện "⭐ Mori đã nhớ: … [Hoàn tác]" 10 s; Hoàn tác = xóa có kiểm chứng (như xóa thường). Panel ⭐ Ghi nhớ: xem/sửa/xóa/xóa toàn bộ.
- Việc nền bị hủy ngay khi người dùng nói/gửi (`cancel_background`), lượt đó không ghi nhớ.
- Kèm theo (cùng lần thử): Mori **tự xưng "Mori"** thay vì "mình" (người dùng cũng xưng "mình" → model nhầm "Mình thích uống gì?" là hỏi về Mori); STT đưa "Morrie/Mô ri…" về "Mori".

## Phương án đã cân nhắc
- Giữ đề xuất + duyệt — người dùng từ chối rõ ràng (phiền, không thông minh).
- Tự đề xuất nhưng vẫn duyệt — vẫn phải vào duyệt.
- Tự nhớ im lặng — người dùng không biết Mori lưu gì về mình; chọn có báo + Hoàn tác để vẫn kiểm soát được.

## Hệ quả
- Đổi SPEC §11.4: "AI chỉ đề xuất" → "Mori tự ghi nhớ, người dùng thấy ngay và hoàn tác/sửa/xóa được".
- Rủi ro: model trích sai/thừa (đo: có lúc ghép ý, sửa lỗi STT chưa trọn — "trên trân châu"); người dùng sửa/xóa trong panel. Thông tin nhạy cảm người dùng kể (sức khỏe, dị ứng…) cũng được lưu — đó là chủ đích, có báo.
- Mỗi lượt thêm một lần gọi model nền (~1–5 s, khi Mori đang nói).
- Audit (2026-11-13): không tự lưu bí mật — prompt trích loại mật khẩu/PIN/OTP/số thẻ/CCCD, thêm lớp chặn trong code (`looks_secret`: từ khóa + dãy ≥ 6 chữ số); không trích đoạn dán nhiều dòng/có link/code; chỉ nhận dòng bắt đầu "Người dùng …"; không tự trích yêu cầu về cách Mori cư xử (đó là việc của lệnh "nhớ là …"). Rủi ro còn lại được chấp nhận: đoạn văn bên thứ ba ngắn, một dòng vẫn có thể gieo ghi nhớ sai — có báo + Hoàn tác.
- Xem lại nếu: model trích sai nhiều (đo trên hội thoại thật), hoặc người dùng muốn bật lại bước duyệt (thêm setting).
