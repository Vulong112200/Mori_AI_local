---
description: Tạo ADR mới từ template trong docs/adr
argument-hint: <tiêu đề quyết định>
---
Tạo Architecture Decision Record cho: **$ARGUMENTS**

1. Liệt kê `docs/adr/`, lấy số lớn nhất + 1 (4 chữ số).
2. Tạo `docs/adr/NNNN-<slug-không-dấu>.md` theo `docs/adr/0000-template.md`, viết tiếng Việt, ngày hôm nay, trạng thái "Đề xuất".
3. Bối cảnh phải dẫn số liệu thật từ `docs/MODEL_EVALUATION.md` / `docs/HARDWARE_REPORT.md` nếu có. Không bịa số. Thiếu dữ liệu → ghi rõ "chưa đo" và đề xuất cách đo.
4. Nêu ít nhất 2 phương án đã cân nhắc.
5. Phần Hệ quả: liệt kê các mục `docs/SPEC.md` cần sửa.
6. Hỏi người dùng chấp nhận không. Chỉ khi người dùng đồng ý: đổi trạng thái thành "Chấp nhận", sửa SPEC.md theo, và tick task ADR tương ứng trong ROADMAP.
7. Nếu ADR thay thế ADR cũ: ghi "Thay thế bởi ADR-NNNN" vào ADR cũ.
