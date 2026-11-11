---
description: Kiểm tra gate của một phase trước khi chuyển phase
argument-hint: <số phase>
---
Kiểm tra gate **Phase $ARGUMENTS** của dự án Mori.

1. Đọc mục "Gate Phase $ARGUMENTS" trong `docs/ROADMAP.md` và các tiêu chí liên quan trong `docs/SPEC.md` (§18, §19).
2. Với **từng** tiêu chí, tìm bằng chứng cụ thể: số đo trong `docs/MODEL_EVALUATION.md`, output test, ADR. Chạy test/lệnh kiểm tra nếu có thể (không tải/cài gì). Không có bằng chứng = chưa đạt.
3. Nếu phase có code (Phase ≥ 1): chạy `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `npx tsc --noEmit`; và chạy agent `agent-skills:security-auditor` để kiểm tra: không có kết nối ra ngoài, service bind 127.0.0.1, không ghi audio ra đĩa, không log nội dung hội thoại, Tauri không bật plugin shell.
4. Báo cáo bảng: tiêu chí · đạt/chưa · bằng chứng.
5. Chỉ khi **tất cả** đạt và người dùng xác nhận: tick các ô gate, đổi trạng thái phase thành "Hoàn thành", phase kế tiếp thành "Đang làm", cập nhật "Phase hiện tại" và ngày. Không tự hạ tiêu chí; muốn đổi tiêu chí phải có ADR.
