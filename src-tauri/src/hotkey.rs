//! Phím gọi Mori toàn cục Ctrl+Space+Q (người dùng chọn — ADR-0013): giữ = đang nói, thả = gửi; UI xử lý
//! bấm nhanh = bật/tắt nghe. Space không phải phím bổ trợ nên RegisterHotKey không đăng ký được → dùng
//! low-level keyboard hook. Hook CHỈ xét phím Q (Ctrl/Space đọc bằng GetAsyncKeyState lúc Q xuống), không lưu/không
//! log phím nào; chỉ "nuốt" phím Q khi đang giữ Ctrl+Space (mọi phím khác đi tiếp bình thường).
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, OnceLock};
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

static TX: OnceLock<mpsc::Sender<bool>> = OnceLock::new();
/// Đang giữ tổ hợp (Q đã xuống khi Ctrl+Space được giữ).
static ACTIVE: AtomicBool = AtomicBool::new(false);

const VK_Q: u32 = 0x51;

/// Quyết định cho một sự kiện phím Q. `held` = Ctrl và Space đang được giữ thật (đọc trạng thái hệ thống, không tự
/// theo dõi: phím nhả trên màn hình UAC/khóa máy không tới hook → cờ tự theo dõi có thể kẹt — audit Phase 7).
/// Trả `(Some(true/false) = báo bắt đầu/kết thúc, nuốt phím?)`.
fn on_q(down: bool, held: bool) -> (Option<bool>, bool) {
    let active = ACTIVE.load(Ordering::SeqCst);
    match (down, active) {
        (true, false) if held => {
            ACTIVE.store(true, Ordering::SeqCst);
            (Some(true), true)
        }
        (true, true) if held => (None, true), // Q lặp khi đang giữ: nuốt, không báo lại
        (true, true) => {
            // Mất sự kiện nhả Q (UAC/khóa máy): kết thúc lượt cũ, Q này là chữ q bình thường.
            ACTIVE.store(false, Ordering::SeqCst);
            (Some(false), false)
        }
        (false, true) => {
            ACTIVE.store(false, Ordering::SeqCst);
            (Some(false), true)
        }
        _ => (None, false),
    }
}

#[cfg(windows)]
unsafe extern "system" fn hook(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_SPACE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_INJECTED, WM_KEYDOWN, WM_KEYUP,
        WM_SYSKEYDOWN, WM_SYSKEYUP,
    };
    if code == HC_ACTION as i32 {
        let msg = wparam as u32;
        let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        // SAFETY: với WH_KEYBOARD_LL, lparam trỏ tới KBDLLHOOKSTRUCT hợp lệ trong lời gọi này.
        let k = unsafe { &*(lparam as *const KBDLLHOOKSTRUCT) };
        // Bỏ phím do chương trình khác giả lập (SendInput) — chỉ người thật bấm mới mở micro.
        if k.vkCode == VK_Q
            && k.flags & LLKHF_INJECTED == 0
            && (down || msg == WM_KEYUP || msg == WM_SYSKEYUP)
        {
            // SAFETY: GetAsyncKeyState chỉ đọc trạng thái phím; bit cao = đang giữ.
            let held = unsafe {
                GetAsyncKeyState(VK_CONTROL as i32) < 0 && GetAsyncKeyState(VK_SPACE as i32) < 0
            };
            let (event, swallow) = on_q(down, held);
            if let (Some(e), Some(tx)) = (event, TX.get()) {
                let _ = tx.send(e); // emit ở thread khác: hook phải trả về thật nhanh
            }
            if swallow {
                return 1;
            }
        }
    }
    // SAFETY: chuyển tiếp đúng tham số nhận được.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

/// Cài hook trên thread riêng (cần vòng lặp message) + thread chuyển sự kiện thành `hotkey-ptt` cho khung chat.
#[cfg(windows)]
pub fn start(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<bool>();
    if TX.set(tx).is_err() {
        return;
    }
    std::thread::spawn(move || {
        for down in rx {
            let _ = app.emit_to("main", "hotkey-ptt", down);
        }
    });
    std::thread::spawn(|| {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetMessageW, SetWindowsHookExW, MSG, WH_KEYBOARD_LL,
        };
        // SAFETY: hook toàn cục cho thread này; hmod null + thread id 0 là cách dùng chuẩn của WH_KEYBOARD_LL.
        let h = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), std::ptr::null_mut(), 0) };
        if h.is_null() {
            warn!("không cài được phím tắt Ctrl+Space+Q");
            return;
        }
        info!("phím tắt Ctrl+Space+Q sẵn sàng");
        // SAFETY: vòng lặp message bắt buộc để Windows gọi hook; MSG được khởi tạo zero.
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {}
    });
}

#[cfg(test)]
mod tests {
    use super::on_q;

    #[test]
    fn chord_detected_and_only_q_swallowed() {
        assert_eq!(on_q(true, false), (None, false), "Q một mình: cho qua");
        assert_eq!(on_q(false, false), (None, false));
        assert_eq!(
            on_q(true, true),
            (Some(true), true),
            "Ctrl+Space+Q → bắt đầu, nuốt Q"
        );
        assert_eq!(
            on_q(true, true),
            (None, true),
            "Q lặp khi giữ: nuốt, không báo lại"
        );
        assert_eq!(
            on_q(false, false),
            (Some(false), true),
            "thả Q (Space đã nhả trước) → kết thúc"
        );
        assert_eq!(
            on_q(true, false),
            (None, false),
            "hết tổ hợp: Q bình thường"
        );
        assert_eq!(on_q(false, false), (None, false));
        // Mất sự kiện nhả Q (UAC/khóa máy) → lần Q kế tiếp không còn Ctrl+Space: kết thúc, không nuốt.
        assert_eq!(on_q(true, true), (Some(true), true));
        assert_eq!(on_q(true, false), (Some(false), false));
        assert_eq!(on_q(false, false), (None, false));
    }
}
