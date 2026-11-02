//! Open a fixed project URL only after an explicit user action.
#![cfg_attr(target_os = "windows", allow(unsafe_code))]

pub const URL: &str = "https://github.com/xinvii/PassGen";

pub fn open() -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
        let url: Vec<u16> = URL.encode_utf16().chain(Some(0)).collect();
        // SAFETY: the null-terminated fixed URL lives through this synchronous call;
        // no shell command, password, or user-provided argument is evaluated.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                windows_sys::w!("open"),
                url.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        if result as isize <= 32 {
            return Err(std::io::Error::other("Could not open the project page"));
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let launcher = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        std::process::Command::new(launcher)
            .arg(URL)
            .spawn()
            .map(|_| ())
    }
}
