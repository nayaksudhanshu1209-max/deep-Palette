use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

pub struct HotkeyEvent {
    pub browser_focused: bool,
}

static EVENTS: OnceLock<Arc<Mutex<Vec<HotkeyEvent>>>> = OnceLock::new();
static CTX: OnceLock<Mutex<Option<eframe::egui::Context>>> = OnceLock::new();
static FALLBACK: AtomicBool = AtomicBool::new(false);

pub fn events() -> Arc<Mutex<Vec<HotkeyEvent>>> {
    EVENTS.get_or_init(|| Arc::new(Mutex::new(Vec::new()))).clone()
}

pub fn install_context(ctx: eframe::egui::Context) {
    *CTX.get_or_init(|| Mutex::new(None)).lock().unwrap() = Some(ctx);
}

pub fn did_fallback() -> bool {
    FALLBACK.load(Ordering::SeqCst)
}

pub fn spawn() {
    let ev = events();
    std::thread::Builder::new()
        .name("hotkey".into())
        .spawn(move || run(ev))
        .ok();
}

#[cfg(windows)]
fn run(ev: Arc<Mutex<Vec<HotkeyEvent>>>) {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let mut mods = MOD_CONTROL;
        let vk = 0x20u32;
        let id = 1;

        if RegisterHotKey(None, id, mods, vk).is_err() {
            crate::logger::warn("Ctrl+Space failed; falling back to Ctrl+Shift+Space");
            FALLBACK.store(true, Ordering::SeqCst);
            mods = MOD_CONTROL | MOD_SHIFT;
            if RegisterHotKey(None, id, mods, vk).is_err() {
                crate::logger::warn("Ctrl+Shift+Space also failed");
                return;
            }
        }

        let mut msg = MSG::default();
        loop {
            if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
                break;
            }
            if msg.message == WM_HOTKEY {
                let browser_focused = is_browser_focused();
                ev.lock().unwrap().push(HotkeyEvent { browser_focused });
                if let Some(ctx) = CTX.get().and_then(|m| m.lock().unwrap().clone()) {
                    ctx.request_repaint();
                }
            }
        }
        let _ = UnregisterHotKey(None, id);
    }
}

#[cfg(windows)]
fn is_browser_focused() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        let root = GetAncestor(fg, GA_ROOT);
        let raw = crate::web::BROWSER_HWND.load(Ordering::SeqCst);
        if raw == 0 {
            return false;
        }
        root.0 as isize == raw
    }
}
