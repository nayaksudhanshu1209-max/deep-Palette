use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{channel, Sender};

pub static BROWSER_HWND: AtomicIsize = AtomicIsize::new(0);

pub enum Cmd {
    Navigate(String),
    Close,
}

pub struct WebHandle {
    tx: Option<Sender<Cmd>>,
}

impl WebHandle {
    pub fn new(safe_mode: bool) -> Self {
        if safe_mode {
            return Self { tx: None };
        }
        let (tx, rx) = channel::<Cmd>();
        std::thread::Builder::new()
            .name("webview".into())
            .spawn(move || {
                if let Err(e) = run(rx) {
                    crate::logger::warn(&format!("web thread: {e}"));
                }
            })
            .ok();
        Self { tx: Some(tx) }
    }

    pub fn navigate(&self, url: String) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Navigate(url));
        }
    }

    #[allow(dead_code)]
    pub fn close(&self) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(Cmd::Close);
        }
    }
}

pub fn resolve(q: &str) -> String {
    let t = q.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        return t.to_string();
    }
    if !t.contains(' ') && t.contains('.') {
        return format!("https://{t}");
    }
    format!("https://www.google.com/search?q={}", t.replace(' ', "+"))
}

#[cfg(windows)]
fn run(rx: std::sync::mpsc::Receiver<Cmd>) -> windows::core::Result<()> {
    use std::sync::{Arc, Mutex};
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    use webview2_com::{
        CreateCoreWebView2ControllerCompletedHandler,
        CreateCoreWebView2EnvironmentCompletedHandler,
    };
    use windows::core::{w, HSTRING, PCWSTR};
    use windows::Win32::Foundation::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;

    unsafe {
        let hinst = GetModuleHandleW(None)?;
        let class = w!("PaletteBrowser");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class,
            w!("Palette Browser"),
            WS_OVERLAPPEDWINDOW & !(WS_CAPTION | WS_THICKFRAME) | WS_POPUP | WS_THICKFRAME,
            200,
            200,
            1000,
            720,
            None,
            None,
            hinst,
            None,
        )?;
        BROWSER_HWND.store(hwnd.0 as isize, Ordering::SeqCst);

        let controller: Arc<Mutex<Option<ICoreWebView2Controller>>> =
            Arc::new(Mutex::new(None));
        let ctrl2 = controller.clone();
        let hwnd_usize = hwnd.0 as usize;

        let env_handler = CreateCoreWebView2EnvironmentCompletedHandler::create(Box::new(
            move |_hr, env| {
                let env = env.ok_or(windows::core::Error::from(E_FAIL))?;
                let hwnd = HWND(hwnd_usize as *mut std::ffi::c_void);
                let ctrl2b = ctrl2.clone();
                let ch = CreateCoreWebView2ControllerCompletedHandler::create(Box::new(
                    move |_hr, c| {
                        let c = c.ok_or(windows::core::Error::from(E_FAIL))?;
                        let mut r = RECT::default();
                        let _ = GetClientRect(hwnd, &mut r);

                        // webview2-com-sys 0.33 exposes the raw COM getters on the
                        // interface struct (Bounds(&self, *mut RECT), IsVisible(&self,
                        // *mut BOOL), etc.) and the setters on the `_Impl` trait
                        // under the same name. UFCS on `_Impl` picks the setter.
                        let _ = <_ as ICoreWebView2Controller_Impl>::Bounds(&c, r);
                        let _ = <_ as ICoreWebView2Controller_Impl>::IsVisible(&c, true);

                        if let Ok(cw) = c.CoreWebView2() {
                            let s = cw.Settings()?;
                            // Same pattern for ICoreWebView2Settings.
                            let _ = <_ as ICoreWebView2Settings_Impl>::AreDevToolsEnabled(&s, false);
                            let _ = <_ as ICoreWebView2Settings_Impl>::AreDefaultContextMenusEnabled(&s, false);
                            let _ = <_ as ICoreWebView2Settings_Impl>::IsStatusBarEnabled(&s, false);
                            let _ = <_ as ICoreWebView2Settings_Impl>::AreHostObjectsAllowed(&s, false);
                            let _ = <_ as ICoreWebView2Settings_Impl>::IsWebMessageEnabled(&s, false);
                            // IsPasswordAutosaveEnabled / IsGeneralAutofillEnabled
                            // live on ICoreWebView2Settings3/4. A fresh user-data
                            // folder already defaults both to false.
                        }
                        *ctrl2b.lock().unwrap() = Some(c);
                        Ok(())
                    },
                ));
                env.CreateCoreWebView2Controller(hwnd, &ch)?;
                Ok(())
            },
        ));

        let user_data = crate::config::localappdata().join("WebView2");
        let ud = HSTRING::from(user_data.to_string_lossy().to_string());
        let _ = CreateCoreWebView2EnvironmentWithOptions(
            PCWSTR::null(),
            &ud,
            None,
            &env_handler,
        );

        let mut msg = MSG::default();
        loop {
            while let Ok(cmd) = rx.try_recv() {
                let guard = controller.lock().unwrap();
                match cmd {
                    Cmd::Navigate(url) => {
                        if let Some(c) = guard.as_ref() {
                            if let Ok(cw) = c.CoreWebView2() {
                                let _ = cw.Navigate(&HSTRING::from(url));
                            }
                        }
                        let _ = ShowWindow(hwnd, SW_SHOW);
                        let _ = SetForegroundWindow(hwnd);
                    }
                    Cmd::Close => {
                        let _ = ShowWindow(hwnd, SW_HIDE);
                    }
                }
            }
            if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_QUIT {
                    break;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(16));
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn wndproc(
    hwnd: windows::Win32::Foundation::HWND,
    msg: u32,
    wp: windows::Win32::Foundation::WPARAM,
    lp: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::*;
    match msg {
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            return windows::Win32::Foundation::LRESULT(0);
        }
        WM_KEYDOWN if wp.0 == 0x1B => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            return windows::Win32::Foundation::LRESULT(0);
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
