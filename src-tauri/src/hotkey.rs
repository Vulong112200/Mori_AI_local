//! Phím gọi Mori toàn cục Ctrl+Space+Q (người dùng chọn — ADR-0013): giữ = đang nói, thả = gửi; UI xử lý
//! bấm nhanh = bật/tắt nghe. Space không phải phím bổ trợ nên RegisterHotKey không đăng ký được → dùng
//! low-level keyboard hook. Hook CHỈ xét 3 phím Ctrl/Space/Q, không lưu/không log phím nào; chỉ "nuốt" phím Q
//! khi đang giữ Ctrl+Space (mọi phím khác đi tiếp bình thường).
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, OnceLock};
use tauri::{AppHandle, Emitter};
use tracing::{info, warn};

static TX: OnceLock<mpsc::Sender<bool>> = OnceLock::new();
static CTRL: AtomicBool = AtomicBool::new(false);
static SPACE: AtomicBool = AtomicBool::new(false);
/// Đang giữ tổ hợp (Q đã xuống khi Ctrl+Space được giữ).
static ACTIVE: AtomicBool = AtomicBool::new(false);

const VK_Q: u32 = 0x51;

/// Quyết định cho một sự kiện phím: `Some(true/false)` = báo bắt đầu/kết thúc; `swallow` = không chuyển tiếp.
fn on_key(vk: u32, down: bool) -> (Option<bool>, bool) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_SPACE,
    };
    match vk {
        v if v == VK_CONTROL as u32 || v == VK_LCONTROL as u32 || v == VK_RCONTROL as u32 => {
            CTRL.store(down, Ordering::SeqCst);
            (None, false)
        }
        v if v == VK_SPACE as u32 => {
            SPACE.store(down, Ordering::SeqCst);
            (None, false)
        }
        VK_Q => {
            let active = ACTIVE.load(Ordering::SeqCst);
            if down && !active && CTRL.load(Ordering::SeqCst) && SPACE.load(Ordering::SeqCst) {
                ACTIVE.store(true, Ordering::SeqCst);
                (Some(true), true)
            } else if !down && active {
                ACTIVE.store(false, Ordering::SeqCst);
                (Some(false), true)
            } else {
                (None, active) // Q lặp khi đang giữ: nuốt; ngoài tổ hợp: cho qua
            }
        }
        _ => (None, false),
    }
}

#[cfg(windows)]
unsafe extern "system" fn hook(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    };
    if code == HC_ACTION as i32 {
        let msg = wparam as u32;
        let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
        if down || msg == WM_KEYUP || msg == WM_SYSKEYUP {
            // SAFETY: với WH_KEYBOARD_LL, lparam trỏ tới KBDLLHOOKSTRUCT hợp lệ trong lời gọi này.
            let vk = unsafe { (*(lparam as *const KBDLLHOOKSTRUCT)).vkCode };
            let (event, swallow) = on_key(vk, down);
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
    use super::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{VK_LCONTROL, VK_SPACE};

    #[test]
    fn chord_detected_and_only_q_swallowed() {
        let (ctrl, space) = (VK_LCONTROL as u32, VK_SPACE as u32);
        assert_eq!(on_key(VK_Q, true), (None, false), "Q một mình: cho qua");
        assert_eq!(on_key(VK_Q, false), (None, false));
        assert_eq!(on_key(ctrl, true), (None, false), "Ctrl không bị nuốt");
        assert_eq!(on_key(space, true), (None, false), "Space không bị nuốt");
        assert_eq!(
            on_key(VK_Q, true),
            (Some(true), true),
            "Ctrl+Space+Q → bắt đầu, nuốt Q"
        );
        assert_eq!(
            on_key(VK_Q, true),
            (None, true),
            "Q lặp khi giữ: nuốt, không báo lại"
        );
        assert_eq!(on_key(space, false), (None, false));
        assert_eq!(on_key(VK_Q, false), (Some(false), true), "thả Q → kết thúc");
        assert_eq!(on_key(ctrl, false), (None, false));
        assert_eq!(
            on_key(VK_Q, true),
            (None, false),
            "hết tổ hợp: Q bình thường"
        );
        assert_eq!(on_key(VK_Q, false), (None, false));
    }
}
