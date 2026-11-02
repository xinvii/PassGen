//! One UI per Windows session; no files, passwords, listener thread or polling.
#![cfg_attr(target_os = "windows", allow(unsafe_code))]

#[cfg(target_os = "windows")]
mod windows {
    use slint::winit_030::{
        WinitWindowAccessor,
        winit::raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };
    use std::{io, ptr, rc::Rc};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM},
        System::Threading::CreateMutexW,
        UI::WindowsAndMessaging::*,
    };

    const ACTIVATE: u32 = WM_APP + 0x504;
    const READY: windows_sys::core::PCWSTR = windows_sys::w!("PassGen.Ready.830C27CE");

    pub struct Instance {
        mutex: HANDLE,
        marker: Rc<Vec<u16>>,
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }

    impl Instance {
        pub fn acquire(preview: bool) -> io::Result<Option<Self>> {
            let name = if preview {
                "Local\\PassGen.Preview.830C27CE"
            } else {
                "Local\\PassGen.App.830C27CE"
            };
            Self::named(name)
        }

        fn named(name: &str) -> io::Result<Option<Self>> {
            let name = wide(name);
            let marker = Rc::new(name.clone());
            // SAFETY: the UTF-16 name lives through CreateMutexW. No initial
            // ownership is requested; retaining the handle keeps the object alive.
            let (mutex, existed) = unsafe {
                let mutex = CreateMutexW(ptr::null(), 0, name.as_ptr());
                let existed = GetLastError() == ERROR_ALREADY_EXISTS;
                (mutex, existed)
            };
            if mutex.is_null() {
                return Err(io::Error::last_os_error());
            }
            if existed {
                notify_existing(&marker);
                // SAFETY: this process owns the newly opened handle, not the mutex.
                unsafe {
                    CloseHandle(mutex);
                }
                return Ok(None);
            }
            Ok(Some(Self { mutex, marker }))
        }

        pub fn attach(&self, window: &slint::Window) {
            window.with_winit_window(|window| {
                let Ok(handle) = window.window_handle() else {
                    return;
                };
                let RawWindowHandle::Win32(handle) = handle.as_raw() else {
                    return;
                };
                // SAFETY: mark only our live main HWND, not a title-matched window
                // from another process. A property value of 1 is an opaque tag.
                unsafe {
                    SetPropW(
                        handle.hwnd.get() as HWND,
                        self.marker.as_ptr(),
                        1usize as HANDLE,
                    );
                }
            });
        }

        pub fn configure(&self, selector: slint::BackendSelector) -> slint::BackendSelector {
            use slint::winit_030::winit::platform::windows::EventLoopBuilderExtWindows;
            let marker = self.marker.clone();
            let mut builder: slint::winit_030::EventLoopBuilder =
                slint::winit_030::winit::event_loop::EventLoop::with_user_event();
            builder.with_msg_hook(move |message| {
                // SAFETY: winit supplies a valid MSG pointer for this callback.
                unsafe {
                    let message = &*message.cast::<MSG>();
                    if message.message != ACTIVATE
                        || GetPropW(message.hwnd, marker.as_ptr()).is_null()
                    {
                        return false;
                    }
                    // During startup the first launch will reveal itself. Never
                    // expose the unfinished native window to service a repeat launch.
                    if !GetPropW(message.hwnd, READY).is_null() {
                        if IsIconic(message.hwnd) != 0 {
                            ShowWindow(message.hwnd, SW_RESTORE);
                        }
                        SetForegroundWindow(message.hwnd);
                    }
                    true
                }
            });
            selector.with_winit_event_loop_builder(builder)
        }
    }

    pub fn mark_ready(hwnd: HWND) {
        // SAFETY: the chrome adapter calls this for its own successfully presented HWND.
        unsafe {
            SetPropW(hwnd, READY, 1usize as HANDLE);
        }
    }

    #[cfg(all(debug_assertions, feature = "ui-qa"))]
    pub fn preview_window_count() -> u32 {
        struct Count {
            name: Vec<u16>,
            windows: u32,
        }
        unsafe extern "system" fn count(hwnd: HWND, context: LPARAM) -> i32 {
            // SAFETY: EnumWindows is synchronous and the Count outlives it.
            unsafe {
                let count = &mut *(context as *mut Count);
                count.windows += u32::from(!GetPropW(hwnd, count.name.as_ptr()).is_null());
                1
            }
        }
        let mut result = Count {
            name: wide("Local\\PassGen.Preview.830C27CE"),
            windows: 0,
        };
        // SAFETY: read-only enumeration of the isolated preview namespace.
        unsafe {
            EnumWindows(Some(count), &mut result as *mut _ as LPARAM);
        }
        result.windows
    }

    fn notify_existing(marker: &[u16]) {
        struct Search<'a> {
            marker: &'a [u16],
            window: HWND,
        }
        unsafe extern "system" fn find(hwnd: HWND, context: LPARAM) -> i32 {
            // SAFETY: EnumWindows is synchronous; the Search stays alive for its call.
            unsafe {
                let search = &mut *(context as *mut Search<'_>);
                if !GetPropW(hwnd, search.marker.as_ptr()).is_null() {
                    search.window = hwnd;
                    return 0;
                }
                1
            }
        }
        let mut search = Search {
            marker,
            window: ptr::null_mut(),
        };
        // SAFETY: only a fixed activation message is posted to a tagged PassGen
        // HWND. No command line, configuration or password crosses processes.
        unsafe {
            EnumWindows(Some(find), &mut search as *mut _ as LPARAM);
            if !search.window.is_null() {
                let mut pid = 0;
                GetWindowThreadProcessId(search.window, &mut pid);
                AllowSetForegroundWindow(pid);
                PostMessageW(search.window, ACTIVATE, 0, 0);
            }
        }
    }

    impl Drop for Instance {
        fn drop(&mut self) {
            // SAFETY: the handle is owned by this guard. The OS also closes it
            // on a crash; there is no stale lock file to remove.
            unsafe {
                CloseHandle(self.mutex);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn only_one_guard_and_reacquire_after_exit() {
            let name = format!("Local\\PassGen.Test.{}.lock", std::process::id());
            let first = Instance::named(&name).unwrap().unwrap();
            assert!(Instance::named(&name).unwrap().is_none());
            drop(first);
            assert!(Instance::named(&name).unwrap().is_some());
        }
        #[test]
        fn independent_namespaces_do_not_block_each_other() {
            let name = format!("Local\\PassGen.Test.{}.separate", std::process::id());
            let _first = Instance::named(&name).unwrap().unwrap();
            let _other = Instance::named(&(name + ".preview")).unwrap().unwrap();
        }
    }
}

#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(not(target_os = "windows"))]
pub struct Instance;
#[cfg(not(target_os = "windows"))]
impl Instance {
    pub fn acquire(_: bool) -> std::io::Result<Option<Self>> {
        Ok(Some(Self))
    }
    pub fn attach(&self, _: &slint::Window) {}
    pub fn configure(&self, selector: slint::BackendSelector) -> slint::BackendSelector {
        selector
    }
}
