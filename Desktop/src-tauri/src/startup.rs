use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use windows::{
    core::w,
    Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0},
        System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LaunchOptions {
    pub background: bool,
    pub preview_only: bool,
}
impl LaunchOptions {
    pub fn from_args(args: impl IntoIterator<Item = String>) -> Self {
        let args: Vec<_> = args.into_iter().collect();
        Self {
            background: args
                .iter()
                .any(|a| a == "--autostart" || a == "--background"),
            preview_only: args.iter().any(|a| a == "--preview-only"),
        }
    }
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
pub struct Instance {
    _mutex: Handle,
    event: Handle,
}
impl Instance {
    pub fn acquire(options: LaunchOptions) -> Result<Option<Self>, String> {
        unsafe {
            // Create the event first: an early second launch can signal before the UI is ready.
            let event = Handle(
                CreateEventW(None, false, false, w!("Local\\F87ProDesktopOpenWindow"))
                    .map_err(|e| e.to_string())?,
            );
            let mutex = Handle(
                CreateMutexW(None, false, w!("Local\\F87ProDesktopLighting-0.1"))
                    .map_err(|e| e.to_string())?,
            );
            if GetLastError() == ERROR_ALREADY_EXISTS {
                if !options.background {
                    SetEvent(event.0).map_err(|e| e.to_string())?;
                }
                return Ok(None);
            }
            Ok(Some(Self {
                _mutex: mutex,
                event,
            }))
        }
    }
    pub fn event_value(&self) -> usize {
        self.event.0 .0 as usize
    }
}
pub struct SignalListener {
    stop: Arc<AtomicBool>,
    event: usize,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl SignalListener {
    pub fn start(event: usize, app: tauri::AppHandle) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = stop.clone();
        let thread = std::thread::Builder::new()
            .name("f87-open-window".into())
            .spawn(move || {
                // The Instance owns this handle until the listener is joined.
                let handle = HANDLE(event as *mut std::ffi::c_void);
                while unsafe { WaitForSingleObject(handle, u32::MAX) } == WAIT_OBJECT_0 {
                    if thread_stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let app_to_show = app.clone();
                    let _ = app.run_on_main_thread(move || {
                        let _ = crate::create_driver(&app_to_show);
                    });
                }
            })
            .expect("open-window listener");
        Self {
            stop,
            event,
            thread: Some(thread),
        }
    }
}
impl Drop for SignalListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        unsafe {
            let _ = SetEvent(HANDLE(self.event as *mut std::ffi::c_void));
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

