#[cfg(windows)]
pub fn shell_open(target: &str) -> bool {
    use windows::core::{w, PCWSTR};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let t: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
    let r = unsafe {
        ShellExecuteW(None, w!("open"), PCWSTR(t.as_ptr()), None, None, SW_SHOWNORMAL)
    };
    r.0 as isize > 32
}

#[cfg(windows)]
pub fn spawn(exe: &str, args: &str, cwd: &str) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut c = std::process::Command::new(exe);
    if !args.is_empty() {
        c.args(split_args(args));
    }
    if !cwd.is_empty() {
        c.current_dir(cwd);
    }
    c.creation_flags(CREATE_NO_WINDOW);
    if let Err(e) = c.spawn() {
        crate::logger::warn(&format!("spawn {exe}: {e}"));
    }
}

fn split_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|x| x.to_string()).collect()
}

#[cfg(windows)]
pub fn below_normal_priority() {
    use windows::Win32::System::Threading::*;
    unsafe {
        let h = GetCurrentThread();
        let _ = SetThreadPriority(h, THREAD_PRIORITY_BELOW_NORMAL);
    }
}

#[cfg(windows)]
pub fn single_instance() -> bool {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::*;
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        let name = HSTRING::from("Local\\Palette_SingleInstance");
        match CreateMutexW(None, false, &name) {
            Ok(_h) => GetLastError() != ERROR_ALREADY_EXISTS,
            Err(_) => true,
        }
    }
}

#[cfg(windows)]
pub fn focus_existing() {
    use windows::core::w;
    use windows::Win32::UI::WindowsAndMessaging::*;
    unsafe {
        if let Ok(h) = FindWindowW(None, w!("Palette")) {
            if !h.0.is_null() {
                let _ = ShowWindow(h, SW_SHOW);
                let _ = SetForegroundWindow(h);
            }
        }
    }
}
