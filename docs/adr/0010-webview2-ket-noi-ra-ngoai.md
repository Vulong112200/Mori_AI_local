# ADR-0010: WebView2 tự kết nối ra Microsoft — giảm bằng cờ, chặn hẳn ở Phase 6

**Trạng thái:** Chấp nhận — đã thực hiện 2026-11-13 (Phase 6)
**Ngày:** 2026-11-12

## Bối cảnh
Audit gate Phase 1 phát hiện tiến trình `msedgewebview2.exe` của Mori (runtime Evergreen dùng chung với Windows) mở HTTPS tới dải IPv6 của Microsoft (`2603:1046:c01:…:443`, `2620:1ec:…:443`). Code Mori và `llama-server` không có kết nối ra ngoài. Vi phạm luật 1 (local-only) và SPEC §19.6 "không có kết nối ra ngoài".

Đo trên máy đích (`Get-NetTCPConnection` mỗi 0.5 s, 200 s từ lúc mở app; mỗi cấu hình 1 lần chạy):

| Cấu hình `additionalBrowserArgs` | Kết nối ra ngoài |
|---|---|
| Mặc định của wry | 2 (Established) |
| + `--disable-background-networking --disable-component-update --disable-domain-reliability --no-pings` | 1 (SynSent) |
| + `--proxy-server=http://127.0.0.1:9` (proxy chết) | 1 (Established) → kết nối không đi qua tầng mạng Chromium |

`--disable-crash-reporter` cũng không có tác dụng (crashpad-handler vẫn chạy). Không thấy dấu hiệu nội dung hội thoại bị gửi; không xác định được endpoint (không có trong DNS cache).

## Quyết định
- Giữ các cờ tắt mạng nền ở `tauri.conf.json` (giảm được 1 kết nối, không có tác dụng phụ). Bỏ cờ proxy (không có tác dụng).
- **Chặn hẳn ở Phase 6** (🔐, cần người dùng duyệt + quyền admin): WebView2 **Fixed Version** đặt trong `E:\Mori Data\runtime\` (`webviewInstallMode`/`browserExecutableFolder`) + luật Windows Firewall chặn outbound cho đúng đường dẫn exe đó. Kiểm lại bằng script đo như trên → 0 kết nối.
- Gate Phase 1 vẫn qua: các tiêu chí của gate 1 trong ROADMAP đều đạt; mục này là ngoại lệ có thời hạn, chặn gate Phase 6.

## Phương án đã cân nhắc
- Chặn firewall cho `msedgewebview2.exe` Evergreen — không được: dùng chung với SearchHost, Widgets, ứng dụng khác.
- Làm ngay bản Fixed Version — người dùng chọn thử cờ trước; tải ~180 MB + admin, hợp với Phase 6 (Model Manager, kiểm SHA-256).

## Hệ quả
- Đến Phase 6, Mori vẫn có 1 kết nối do Microsoft khởi tạo lúc mở app → ARCHITECTURE ghi rõ, không tuyên bố "0 kết nối" cho app.
- Phase 6 thêm task "WebView2 Fixed Version + firewall"; gate 6 kiểm 0 kết nối.
- Xem lại nếu Microsoft cung cấp tùy chọn tắt (`CoreWebView2EnvironmentOptions`) mà wry expose.

## Thực hiện (2026-11-13, người dùng duyệt tải + quyền admin)
- WebView2 **Fixed Version 157.0.4341.57 x64** (cab 311 MB, `sha256 9a7ac986…c01f`, chữ ký Microsoft hợp lệ) → `E:\Mori Data\runtime\webview2\`. App đặt `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` lúc khởi động (`use_fixed_webview2`, lib.rs); thiếu thư mục thì quay về Evergreen + log cảnh báo.
- `scripts/firewall.ps1` (chạy admin): 8 luật **Outbound Block** theo đường dẫn chương trình — msedgewebview2.exe (bản Fixed), llama-server (vulkan + cpu), whisper-server, python (venv + interpreter), mori.exe (debug + release). Loopback không bị Windows Firewall lọc → Mori chạy bình thường.
- Đo khi app chạy: 12 tiến trình Mori, chỉ nghe `127.0.0.1`, **0 kết nối TCP ra ngoài loopback**, 0 UDP; 8 socket của WebView2 ở trạng thái `Bound` (không kết nối được tới đâu — bị chặn). Trước đó (Evergreen, chưa chặn): 1–2 kết nối HTTPS tới Microsoft.
- Đổi phiên bản Fixed / build mới của mori.exe ở đường dẫn khác → chạy lại `scripts/firewall.ps1`.
