use regex::Regex;
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HWND, LPARAM, RECT, WAIT_OBJECT_0, WAIT_TIMEOUT, WPARAM,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::TerminateProcess;
use windows::Win32::System::Threading::{
    CreateProcessW, GetExitCodeProcess, OpenProcess, PROCESS_CREATION_FLAGS, PROCESS_INFORMATION,
    PROCESS_NAME_NATIVE, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE, QueryFullProcessImageNameW,
    STARTUPINFOW, WaitForSingleObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, IsZoomed, SW_MAXIMIZE, SW_MINIMIZE,
    SW_RESTORE, SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetForegroundWindow,
    SetWindowPos, ShowWindow, WM_CLOSE,
};
use windows::core::BOOL;
use windows::core::{PCWSTR, PWSTR};

const STILL_ACTIVE: u32 = 259;

#[derive(Debug)]
pub enum AutomationError {
    Win32Failure { code: i32 },
    ApplicationNotFound { name: String },
    WindowNotFound { title: String },
    ProcessTerminated { pid: u32 },
    ProcessNotFound { name: String },
    AccessDenied { operation: String },
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationId(u32);

pub struct Application {
    pid: u32,
    handle: HANDLE,
}

impl Drop for Application {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

impl Application {
    #[must_use]
    pub const fn id(&self) -> ApplicationId {
        ApplicationId(self.pid)
    }

    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    #[must_use]
    pub fn is_running(&self) -> bool {
        let mut exit_code: u32 = 0;
        unsafe {
            let result = GetExitCodeProcess(self.handle, &raw mut exit_code);
            result.is_ok() && exit_code == STILL_ACTIVE
        }
    }

    pub fn close(&self) -> Result<(), AutomationError> {
        if !self.is_running() {
            return Err(AutomationError::ProcessTerminated { pid: self.pid });
        }

        unsafe {
            let result = TerminateProcess(self.handle, 1);
            match result {
                Ok(()) => Ok(()),
                Err(e) => Err(AutomationError::Win32Failure { code: e.code().0 }),
            }
        }
    }

    pub fn wait_for_exit(&self, timeout_ms: Option<u32>) -> Result<u32, AutomationError> {
        let timeout = timeout_ms.unwrap_or(u32::MAX);
        unsafe {
            let result = WaitForSingleObject(self.handle, timeout);

            match result {
                WAIT_OBJECT_0 => {
                    let mut exit_code = 0;
                    GetExitCodeProcess(self.handle, &raw mut exit_code)
                        .map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;
                    Ok(exit_code)
                }
                WAIT_TIMEOUT => Err(AutomationError::Other("Wait timeout".to_string())),
                _ => Err(AutomationError::Win32Failure {
                    code: GetLastError().0.cast_signed(),
                }),
            }
        }
    }

    pub fn get_name(&self) -> Result<String, AutomationError> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            let snapshot =
                snapshot.map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            let result = Process32FirstW(snapshot, &raw mut entry);
            if result.is_err() {
                let _ = CloseHandle(snapshot);
                return Err(AutomationError::Win32Failure {
                    code: GetLastError().0.cast_signed(),
                });
            }

            loop {
                if entry.th32ProcessID == self.pid {
                    let process_name = {
                        let end_pos = entry
                            .szExeFile
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(entry.szExeFile.len());
                        String::from_utf16_lossy(&entry.szExeFile[..end_pos])
                    };
                    let _ = CloseHandle(snapshot);
                    return Ok(process_name);
                }

                if Process32NextW(snapshot, &raw mut entry).is_err() {
                    break;
                }
            }

            let _ = CloseHandle(snapshot);
        }

        Err(AutomationError::ProcessNotFound {
            name: format!("PID {}", self.pid),
        })
    }

    pub fn get_path(&self) -> Result<String, AutomationError> {
        unsafe {
            let mut buffer = [0u16; 260]; // MAX_PATH
            let mut size = buffer.len() as u32;

            let result = QueryFullProcessImageNameW(
                self.handle,
                PROCESS_NAME_NATIVE,
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &raw mut size,
            );

            if result.is_ok() {
                let path = String::from_utf16_lossy(&buffer[..size as usize]);
                Ok(path)
            } else {
                Err(AutomationError::Win32Failure {
                    code: GetLastError().0.cast_signed(),
                })
            }
        }
    }

    pub fn get_exit_code(&self) -> Result<u32, AutomationError> {
        let mut exit_code: u32 = 0;
        unsafe {
            let result = GetExitCodeProcess(self.handle, &raw mut exit_code);
            result.map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;
            Ok(exit_code)
        }
    }

    pub fn get_parent_pid(&self) -> Result<u32, AutomationError> {
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            let snapshot =
                snapshot.map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

            let mut entry = PROCESSENTRY32W::default();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

            let result = Process32FirstW(snapshot, &raw mut entry);
            if result.is_err() {
                let _ = CloseHandle(snapshot);
                return Err(AutomationError::Win32Failure {
                    code: GetLastError().0.cast_signed(),
                });
            }

            loop {
                if entry.th32ProcessID == self.pid {
                    let parent_pid = entry.th32ParentProcessID;
                    let _ = CloseHandle(snapshot);
                    return Ok(parent_pid);
                }

                if Process32NextW(snapshot, &raw mut entry).is_err() {
                    break;
                }
            }

            let _ = CloseHandle(snapshot);
        }

        Err(AutomationError::ProcessNotFound {
            name: format!("PID {}", self.pid),
        })
    }

    pub fn kill(&self, exit_code: u32) -> Result<(), AutomationError> {
        if !self.is_running() {
            return Err(AutomationError::ProcessTerminated { pid: self.pid });
        }

        unsafe {
            let result = TerminateProcess(self.handle, exit_code);
            match result {
                Ok(()) => Ok(()),
                Err(e) => Err(AutomationError::Win32Failure { code: e.code().0 }),
            }
        }
    }
}

pub fn launch_application(exe: &str, args: &str) -> Result<Application, AutomationError> {
    let mut si = STARTUPINFOW::default();
    si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;

    let mut pi = PROCESS_INFORMATION::default();

    let cmd = format!("\"{exe}\" {args}");
    let mut cmd_w: Vec<u16> = cmd.encode_utf16().chain(Some(0)).collect();

    let success = unsafe {
        CreateProcessW(
            PCWSTR::null(),
            Some(PWSTR(cmd_w.as_mut_ptr())),
            None,
            None,
            false,
            PROCESS_CREATION_FLAGS(0),
            None,
            PCWSTR::null(),
            &raw const si,
            &raw mut pi,
        )
    };

    if success.is_err() {
        return Err(AutomationError::Win32Failure {
            code: unsafe { GetLastError().0.cast_signed() },
        });
    }

    unsafe {
        let _ = CloseHandle(pi.hThread);
    }

    Ok(Application {
        pid: pi.dwProcessId,
        handle: pi.hProcess,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowId(pub isize);

impl WindowId {
    const fn as_hwnd(&self) -> HWND {
        HWND(self.0 as *mut core::ffi::c_void)
    }

    fn from_hwnd(hwnd: HWND) -> Self {
        Self(hwnd.0 as isize)
    }
}

#[derive(Debug)]
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub class_name: String,
    pub bounds: Rect,
    pub visible: bool,
}

impl Window {
    pub fn activate(&self) -> Result<(), AutomationError> {
        unsafe {
            let _ = SetForegroundWindow(self.id.as_hwnd());
            Ok(())
        }
    }

    pub fn close(&self) -> Result<(), AutomationError> {
        unsafe {
            SendMessageW(
                self.id.as_hwnd(),
                WM_CLOSE,
                Some(WPARAM(0)),
                Some(LPARAM(0)),
            );
            Ok(())
        }
    }

    pub fn minimize(&self) -> Result<(), AutomationError> {
        unsafe {
            let _ = ShowWindow(self.id.as_hwnd(), SW_MINIMIZE);
            Ok(())
        }
    }

    pub fn maximize(&self) -> Result<(), AutomationError> {
        unsafe {
            let _ = ShowWindow(self.id.as_hwnd(), SW_MAXIMIZE);
            Ok(())
        }
    }

    pub fn restore(&self) -> Result<(), AutomationError> {
        unsafe {
            let _ = ShowWindow(self.id.as_hwnd(), SW_RESTORE);
            Ok(())
        }
    }

    pub fn show(&self) -> Result<(), AutomationError> {
        unsafe {
            let _ = ShowWindow(self.id.as_hwnd(), SW_SHOW);
            Ok(())
        }
    }

    pub fn resize(&self, width: i32, height: i32) -> Result<(), AutomationError> {
        unsafe {
            SetWindowPos(
                self.id.as_hwnd(),
                None,
                0,
                0,
                width,
                height,
                SWP_NOMOVE | SWP_NOZORDER,
            )
            .map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;
            Ok(())
        }
    }

    pub fn move_to(&self, x: i32, y: i32) -> Result<(), AutomationError> {
        unsafe {
            SetWindowPos(
                self.id.as_hwnd(),
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER,
            )
            .map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;
            Ok(())
        }
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.id.as_hwnd()).as_bool() }
    }

    #[must_use]
    pub fn is_minimized(&self) -> bool {
        unsafe { IsIconic(self.id.as_hwnd()).as_bool() }
    }

    #[must_use]
    pub fn is_maximized(&self) -> bool {
        unsafe { IsZoomed(self.id.as_hwnd()).as_bool() }
    }

    pub fn refresh(&mut self) -> Result<(), AutomationError> {
        unsafe {
            let updated = create_window_from_hwnd(self.id.as_hwnd())?;
            self.title = updated.title;
            self.class_name = updated.class_name;
            self.bounds = updated.bounds;
            self.visible = updated.visible;
            Ok(())
        }
    }

    #[must_use]
    pub fn get_process_id(&self) -> u32 {
        unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(self.id.as_hwnd(), Some(&raw mut pid));
            pid
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub fn find_processes_by_name(name: &str) -> Result<Vec<Application>, AutomationError> {
    let mut applications = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        let snapshot = snapshot.map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

        let mut entry = PROCESSENTRY32W::default();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &raw mut entry).is_ok() {
            loop {
                let process_name = {
                    let end_pos = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    String::from_utf16_lossy(&entry.szExeFile[..end_pos])
                };

                if process_name.to_lowercase().contains(&name.to_lowercase())
                    && let Ok(app) = attach_to_process_by_pid(entry.th32ProcessID)
                {
                    applications.push(app);
                }

                if Process32NextW(snapshot, &raw mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    Ok(applications)
}

pub fn attach_to_process_by_pid(pid: u32) -> Result<Application, AutomationError> {
    unsafe {
        let access_rights = PROCESS_QUERY_INFORMATION | PROCESS_TERMINATE;
        let handle = OpenProcess(access_rights, false, pid);

        let handle = handle.map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

        Ok(Application { pid, handle })
    }
}

pub fn attach_to_process_by_name(name: &str) -> Result<Application, AutomationError> {
    let processes = find_processes_by_name(name)?;
    if processes.is_empty() {
        return Err(AutomationError::ProcessNotFound {
            name: name.to_string(),
        });
    }

    Ok(processes.into_iter().next().unwrap())
}

unsafe fn get_window_text(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut buffer);
        if len > 0 {
            String::from_utf16_lossy(&buffer[..len as usize])
        } else {
            String::new()
        }
    }
}

unsafe fn get_window_class(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut buffer);
        if len > 0 {
            String::from_utf16_lossy(&buffer[..len as usize])
        } else {
            String::new()
        }
    }
}

unsafe fn get_window_bounds(hwnd: HWND) -> Result<Rect, AutomationError> {
    unsafe {
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &raw mut rect)
            .map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

        Ok(Rect {
            left: rect.left,
            top: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        })
    }
}

unsafe fn create_window_from_hwnd(hwnd: HWND) -> Result<Window, AutomationError> {
    unsafe {
        let title = get_window_text(hwnd);
        let class_name = get_window_class(hwnd);
        let bounds = get_window_bounds(hwnd)?;
        let visible = IsWindowVisible(hwnd).as_bool();

        Ok(Window {
            id: WindowId::from_hwnd(hwnd),
            title,
            class_name,
            bounds,
            visible,
        })
    }
}

struct EnumWindowsData {
    windows: Vec<Window>,
}

unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut EnumWindowsData);

        if let Ok(window) = create_window_from_hwnd(hwnd) {
            data.windows.push(window);
        }

        BOOL(1)
    }
}

pub fn find_windows() -> Result<Vec<Window>, AutomationError> {
    unsafe {
        let mut data = EnumWindowsData {
            windows: Vec::new(),
        };

        let lparam = LPARAM(&raw mut data as isize);
        EnumWindows(Some(enum_windows_callback), lparam)
            .map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

        Ok(data.windows)
    }
}

pub fn find_windows_by_title(title: &str) -> Result<Vec<Window>, AutomationError> {
    let windows = find_windows()?;
    Ok(windows
        .into_iter()
        .filter(|w| w.title.to_lowercase().contains(&title.to_lowercase()))
        .collect())
}

pub fn find_windows_by_title_regex(pattern: &str) -> Result<Vec<Window>, AutomationError> {
    let regex = Regex::new(pattern)
        .map_err(|e| AutomationError::Other(format!("Invalid regex pattern: {e}")))?;

    let windows = find_windows()?;
    Ok(windows
        .into_iter()
        .filter(|w| regex.is_match(&w.title))
        .collect())
}

pub fn find_windows_by_class(class_name: &str) -> Result<Vec<Window>, AutomationError> {
    let windows = find_windows()?;
    Ok(windows
        .into_iter()
        .filter(|w| {
            w.class_name
                .to_lowercase()
                .contains(&class_name.to_lowercase())
        })
        .collect())
}

pub fn find_windows_by_process(pid: u32) -> Result<Vec<Window>, AutomationError> {
    let windows = find_windows()?;
    Ok(windows
        .into_iter()
        .filter(|w| unsafe {
            let mut window_pid = 0u32;
            GetWindowThreadProcessId(w.id.as_hwnd(), Some(&raw mut window_pid));
            window_pid == pid
        })
        .collect())
}

pub fn get_foreground_window() -> Result<Window, AutomationError> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return Err(AutomationError::WindowNotFound {
                title: "No foreground window".to_string(),
            });
        }
        create_window_from_hwnd(hwnd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_creation() {
        let result = launch_application("notepad.exe", "");
        assert!(result.is_ok());

        let app = result.unwrap();
        assert!(app.is_running());
        assert_eq!(app.pid(), app.id().0);

        // Test get_name method
        let name_result = app.get_name();
        assert!(name_result.is_ok());
        let name = name_result.unwrap();
        assert!(name.to_lowercase().contains("notepad"));

        app.close().unwrap();
        assert!(!app.is_running());

        let result = app.wait_for_exit(Some(5000));
        assert!(result.is_ok());

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_process_enumeration() {
        let result = launch_application("notepad.exe", "");
        assert!(result.is_ok());

        let processes = find_processes_by_name("notepad");
        assert!(processes.is_ok());

        let procs = processes.unwrap();
        assert!(!procs.is_empty());

        for proc in procs {
            assert!(proc.is_running());
        }

        let app = result.unwrap();
        app.close().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_attach_to_process() {
        let processes = find_processes_by_name("notepad").unwrap();
        if let Some(proc) = processes.first() {
            let attached = attach_to_process_by_pid(proc.pid());
            assert!(attached.is_ok());
            let attached_app = attached.unwrap();
            assert_eq!(attached_app.pid(), proc.pid());
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_attach_to_process_by_name() {
        let result = launch_application("notepad.exe", "");
        assert!(result.is_ok());

        let result = attach_to_process_by_name("notepad");
        assert!(result.is_ok());

        std::thread::sleep(std::time::Duration::from_millis(100));

        let app = result.unwrap();
        assert!(app.is_running());

        app.close().unwrap();
        assert!(!app.is_running());

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_find_windows() {
        let result = find_windows();
        assert!(result.is_ok());

        let windows = result.unwrap();
        assert!(!windows.is_empty());
    }

    #[test]
    fn test_find_windows_by_title() {
        let app = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_title("notepad").unwrap();
        assert!(!windows.is_empty());

        let notepad_window = windows.first().unwrap();
        assert!(notepad_window.title.to_lowercase().contains("notepad"));
        assert!(notepad_window.visible);

        app.close().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_find_windows_by_title_regex() {
        let app = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_title_regex(r"(?i).*notepad.*").unwrap();
        assert!(!windows.is_empty());

        let windows_exact = find_windows_by_title_regex(r"^Untitled - Notepad$").unwrap();
        assert!(!windows_exact.is_empty());

        app.close().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_find_windows_by_class() {
        let app = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_class("Notepad").unwrap();
        assert!(!windows.is_empty());

        app.close().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_find_windows_by_process() {
        let app = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_process(app.pid()).unwrap();
        assert!(!windows.is_empty());

        let window = windows.first().unwrap();
        assert_eq!(window.get_process_id(), app.pid());

        app.close().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_window_operations() {
        let _ = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_title("notepad").unwrap();
        assert!(!windows.is_empty());

        let window = windows.first().unwrap();

        assert!(window.activate().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));

        assert!(window.minimize().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(window.is_minimized());

        assert!(window.restore().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(!window.is_minimized());

        assert!(window.maximize().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(window.is_maximized());

        assert!(window.restore().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert!(!window.is_maximized());

        assert!(window.close().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_window_resize_and_move() {
        let _ = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let mut windows = find_windows_by_title("notepad").unwrap();
        assert!(!windows.is_empty());

        let window = windows.first_mut().unwrap();

        assert!(window.resize(800, 600).is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));

        assert!(window.refresh().is_ok());
        assert_eq!(window.bounds.width, 800);
        assert_eq!(window.bounds.height, 600);

        assert!(window.move_to(100, 100).is_ok());
        std::thread::sleep(std::time::Duration::from_millis(200));

        assert!(window.refresh().is_ok());
        assert_eq!(window.bounds.left, 100);
        assert_eq!(window.bounds.top, 100);

        assert!(window.close().is_ok());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_get_foreground_window() {
        let app = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_title("notepad").unwrap();
        let window = windows.first().unwrap();

        window.activate().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));

        let foreground = get_foreground_window().unwrap();
        assert!(foreground.title.to_lowercase().contains("notepad"));

        app.close().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    #[test]
    fn test_window_refresh() {
        let app = launch_application("notepad.exe", "").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));

        let windows = find_windows_by_title("notepad").unwrap();
        let mut window = windows.into_iter().next().unwrap();

        let original_title = window.title.clone();

        window.minimize().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));

        window.refresh().unwrap();
        assert!(window.is_minimized());

        window.restore().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));

        window.refresh().unwrap();
        assert!(!window.is_minimized());
        assert_eq!(window.title, original_title);

        app.close().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
