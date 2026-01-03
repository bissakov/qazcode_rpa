use regex::Regex;
use std::ffi::c_void;
use std::fmt::{self, Display, Formatter};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub mod generator;
pub mod selector;
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
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_TYPE, KEYBD_EVENT_FLAGS, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN,
    MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_WHEEL, MOUSEINPUT, SendInput, VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BM_GETCHECK, BM_SETCHECK, EnumChildWindows, EnumWindows, GetClassNameW, GetCursorPos,
    GetForegroundWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, IsZoomed, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE, SW_SHOW, SWP_NOMOVE,
    SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetForegroundWindow, SetWindowPos, ShowWindow,
    WM_CLOSE, WM_GETTEXT, WM_SETTEXT,
};
use windows::core::BOOL;
use windows::core::{PCWSTR, PWSTR};

const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;
const INPUT_MOUSE: INPUT_TYPE = INPUT_TYPE(0);
const INPUT_KEYBOARD: INPUT_TYPE = INPUT_TYPE(1);

const STILL_ACTIVE: u32 = 259;

const SB_LINEUP: usize = 0;
const SB_LINEDOWN: usize = 1;
const SB_LINELEFT: usize = 0;
const SB_LINERIGHT: usize = 1;
const WM_SCROLL: u32 = 0x0114;
const WHEEL_DELTA: i32 = 120;
const EM_SETSEL: u32 = 0x00B1;
const EM_GETSEL: u32 = 0x00B0;

const VK_BACKSPACE: u16 = 0x08;
const VK_TAB: u16 = 0x09;
const VK_RETURN: u16 = 0x0D;
const VK_SHIFT: u16 = 0x10;
const VK_CONTROL: u16 = 0x11;
const VK_ALT: u16 = 0x12;
const VK_ESCAPE: u16 = 0x1B;
const VK_SPACE: u16 = 0x20;
const VK_DELETE: u16 = 0x2E;
const VK_INSERT: u16 = 0x2D;
const VK_HOME: u16 = 0x24;
const VK_END: u16 = 0x23;
const VK_PAGE_UP: u16 = 0x21;
const VK_PAGE_DOWN: u16 = 0x22;
const VK_LEFT: u16 = 0x25;
const VK_UP: u16 = 0x26;
const VK_RIGHT: u16 = 0x27;
const VK_DOWN: u16 = 0x28;
#[allow(dead_code)]
const VK_F1: u16 = 0x70;
#[allow(dead_code)]
const VK_F2: u16 = 0x71;
#[allow(dead_code)]
const VK_F3: u16 = 0x72;
#[allow(dead_code)]
const VK_F4: u16 = 0x73;
#[allow(dead_code)]
const VK_F5: u16 = 0x74;
#[allow(dead_code)]
const VK_F6: u16 = 0x75;
#[allow(dead_code)]
const VK_F7: u16 = 0x76;
#[allow(dead_code)]
const VK_F8: u16 = 0x77;
#[allow(dead_code)]
const VK_F9: u16 = 0x78;
#[allow(dead_code)]
const VK_F10: u16 = 0x79;
#[allow(dead_code)]
const VK_F11: u16 = 0x7A;
#[allow(dead_code)]
const VK_F12: u16 = 0x7B;

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

impl Display for AutomationError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            AutomationError::Win32Failure { code } => {
                let msg = error_code_to_message(*code);
                write!(f, "Win32 error {}: {}", code, msg)
            }
            AutomationError::ApplicationNotFound { name } => {
                write!(f, "Application '{}' not found in running processes", name)
            }
            AutomationError::WindowNotFound { title } => {
                write!(f, "Window with title containing '{}' not found", title)
            }
            AutomationError::ProcessTerminated { pid } => {
                write!(f, "Process with PID {} has terminated", pid)
            }
            AutomationError::ProcessNotFound { name } => {
                write!(f, "Process '{}' not found", name)
            }
            AutomationError::AccessDenied { operation } => {
                write!(f, "Access denied for operation: {}", operation)
            }
            AutomationError::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

fn error_code_to_message(code: i32) -> &'static str {
    match code {
        0 => "Success (no error)",
        2 => "File or path not found",
        5 => "Access denied",
        6 => "Invalid handle",
        8 => "Not enough memory",
        11 => "Invalid environment",
        13 => "Invalid data",
        126 => "Application not found",
        127 => "Application path not found",
        740 => "Elevated privileges required",
        1400 => "Invalid window handle",
        1401 => "Invalid menu handle",
        1402 => "Invalid cursor handle",
        1404 => "Invalid accelerator table handle",
        1407 => "No state information for window",
        _ => "Unknown error",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationId(pub u32);

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
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
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
                PWSTR(buffer.as_mut_ptr()),
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
            entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

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
    si.cb = size_of::<STARTUPINFOW>() as u32;

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
    pub const fn as_hwnd(&self) -> HWND {
        HWND(self.0 as *mut c_void)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlId(pub isize);

impl ControlId {
    pub const fn as_hwnd(&self) -> HWND {
        HWND(self.0 as *mut c_void)
    }

    fn from_hwnd(hwnd: HWND) -> Self {
        Self(hwnd.0 as isize)
    }
}

#[derive(Debug)]
pub struct Control {
    pub id: ControlId,
    pub class_name: String,
    pub text: String,
    pub bounds: Rect,
    pub visible: bool,
    pub enabled: bool,
}

impl Control {
    fn as_hwnd(&self) -> HWND {
        self.id.as_hwnd()
    }

    pub fn click(&self) -> Result<(), AutomationError> {
        let center_x = self.bounds.left + self.bounds.width / 2;
        let center_y = self.bounds.top + self.bounds.height / 2;
        click(center_x, center_y)
    }

    pub fn right_click(&self) -> Result<(), AutomationError> {
        let center_x = self.bounds.left + self.bounds.width / 2;
        let center_y = self.bounds.top + self.bounds.height / 2;
        right_click(center_x, center_y)
    }

    pub fn double_click(&self) -> Result<(), AutomationError> {
        let center_x = self.bounds.left + self.bounds.width / 2;
        let center_y = self.bounds.top + self.bounds.height / 2;
        double_click(center_x, center_y)
    }

    pub fn focus(&self) -> Result<(), AutomationError> {
        unsafe {
            let _ = SetForegroundWindow(self.as_hwnd());
            Ok(())
        }
    }

    pub fn set_text(&self, text: &str) -> Result<(), AutomationError> {
        let text_w: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        unsafe {
            SendMessageW(
                self.as_hwnd(),
                WM_SETTEXT,
                Some(WPARAM(0)),
                Some(LPARAM(text_w.as_ptr() as isize)),
            );
            Ok(())
        }
    }

    pub fn get_text(&self) -> Result<String, AutomationError> {
        unsafe {
            let mut buffer = [0u16; 1024];
            let len = SendMessageW(
                self.as_hwnd(),
                WM_GETTEXT,
                Some(WPARAM(buffer.len())),
                Some(LPARAM(buffer.as_mut_ptr() as isize)),
            )
            .0 as usize;

            if len > 0 {
                Ok(String::from_utf16_lossy(&buffer[..len]).to_string())
            } else {
                Ok(String::new())
            }
        }
    }

    pub fn is_checked(&self) -> Result<bool, AutomationError> {
        unsafe {
            let result = SendMessageW(self.as_hwnd(), BM_GETCHECK, None, None).0 as usize;

            Ok(result == BST_CHECKED as usize)
        }
    }

    pub fn set_checked(&self, checked: bool) -> Result<(), AutomationError> {
        unsafe {
            let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
            SendMessageW(self.as_hwnd(), BM_SETCHECK, Some(WPARAM(state)), None);
            Ok(())
        }
    }

    pub fn toggle_checkbox(&self) -> Result<(), AutomationError> {
        let current = self.is_checked()?;
        self.set_checked(!current)
    }

    pub fn clear_text(&self) -> Result<(), AutomationError> {
        self.focus()?;
        key_down_ctrl()?;
        sleep(Duration::from_millis(50));
        press_key('a')?;
        sleep(Duration::from_millis(50));
        key_up_ctrl()?;
        sleep(Duration::from_millis(50));
        press_delete()?;
        Ok(())
    }

    pub fn copy_to_clipboard(&self) -> Result<(), AutomationError> {
        self.focus()?;
        key_down_ctrl()?;
        sleep(Duration::from_millis(50));
        press_key('a')?;
        sleep(Duration::from_millis(50));
        key_up_ctrl()?;
        sleep(Duration::from_millis(50));
        key_down_ctrl()?;
        sleep(Duration::from_millis(50));
        press_key('c')?;
        sleep(Duration::from_millis(50));
        key_up_ctrl()?;
        Ok(())
    }

    pub fn paste_from_clipboard(&self) -> Result<(), AutomationError> {
        self.focus()?;
        key_down_ctrl()?;
        sleep(Duration::from_millis(50));
        press_key('v')?;
        sleep(Duration::from_millis(50));
        key_up_ctrl()?;
        Ok(())
    }

    pub fn select_text(&self, start: i32, length: i32) -> Result<(), AutomationError> {
        unsafe {
            SendMessageW(
                self.as_hwnd(),
                EM_SETSEL,
                Some(WPARAM(start as usize)),
                Some(LPARAM(start as isize + length as isize)),
            );
            Ok(())
        }
    }

    pub fn get_selected_text(&self) -> Result<String, AutomationError> {
        unsafe {
            let result = SendMessageW(self.as_hwnd(), EM_GETSEL, None, None).0 as isize;
            let start = result & 0xFFFF;
            let end = (result >> 16) & 0xFFFF;

            if start == end {
                return Ok(String::new());
            }

            let full_text = self.get_text()?;
            let start_idx = start.max(0) as usize;
            let end_idx = end.max(0) as usize;

            if start_idx >= full_text.len() || end_idx > full_text.len() {
                return Ok(String::new());
            }

            Ok(full_text[start_idx..end_idx.min(full_text.len())].to_string())
        }
    }

    pub fn refresh(&mut self) -> Result<(), AutomationError> {
        unsafe {
            let updated = create_control_from_hwnd(self.as_hwnd())?;
            self.class_name = updated.class_name;
            self.text = updated.text;
            self.bounds = updated.bounds;
            self.visible = updated.visible;
            self.enabled = updated.enabled;
            Ok(())
        }
    }
}

pub fn find_processes_by_name(name: &str) -> Result<Vec<Application>, AutomationError> {
    let mut applications = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        let snapshot = snapshot.map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;

        let mut entry = PROCESSENTRY32W::default();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;

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

unsafe fn create_control_from_hwnd(hwnd: HWND) -> Result<Control, AutomationError> {
    unsafe {
        let class_name = get_window_class(hwnd);
        let text = get_window_text(hwnd);
        let bounds = get_window_bounds(hwnd)?;
        let visible = IsWindowVisible(hwnd).as_bool();
        let enabled = true;

        Ok(Control {
            id: ControlId::from_hwnd(hwnd),
            class_name,
            text,
            bounds,
            visible,
            enabled,
        })
    }
}

struct EnumControlsData {
    controls: Vec<Control>,
}

unsafe extern "system" fn enum_child_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let data = &mut *(lparam.0 as *mut EnumControlsData);

        if let Ok(control) = create_control_from_hwnd(hwnd) {
            data.controls.push(control);
        }

        BOOL(1)
    }
}

pub fn find_controls_in_window(parent_hwnd: HWND) -> Result<Vec<Control>, AutomationError> {
    unsafe {
        let mut data = EnumControlsData {
            controls: Vec::new(),
        };

        let lparam = LPARAM(&raw mut data as isize);
        let _ = EnumChildWindows(Some(parent_hwnd), Some(enum_child_windows_callback), lparam);

        Ok(data.controls)
    }
}

pub fn find_controls_by_class(
    parent_hwnd: HWND,
    class_name: &str,
) -> Result<Vec<Control>, AutomationError> {
    let controls = find_controls_in_window(parent_hwnd)?;
    Ok(controls
        .into_iter()
        .filter(|c| {
            c.class_name
                .to_lowercase()
                .contains(&class_name.to_lowercase())
        })
        .collect())
}

pub fn find_control_by_text(parent_hwnd: HWND, text: &str) -> Result<Control, AutomationError> {
    let controls = find_controls_in_window(parent_hwnd)?;
    controls
        .into_iter()
        .find(|c| c.text.to_lowercase().contains(&text.to_lowercase()))
        .ok_or(AutomationError::WindowNotFound {
            title: format!("Control with text '{}'", text),
        })
}

pub fn find_control_by_text_exact(
    parent_hwnd: HWND,
    text: &str,
) -> Result<Control, AutomationError> {
    let controls = find_controls_in_window(parent_hwnd)?;
    controls
        .into_iter()
        .find(|c| c.text == text)
        .ok_or(AutomationError::WindowNotFound {
            title: format!("Control with exact text '{}'", text),
        })
}

pub fn find_control_by_class_exact(
    parent_hwnd: HWND,
    class_name: &str,
) -> Result<Control, AutomationError> {
    let controls = find_controls_in_window(parent_hwnd)?;
    controls
        .into_iter()
        .find(|c| c.class_name == class_name)
        .ok_or(AutomationError::WindowNotFound {
            title: format!("Control with class '{}'", class_name),
        })
}

pub fn find_window_by_selector(dsl: &str) -> Result<Window, AutomationError> {
    let selector = selector::Selector::parse(dsl)?;
    find_window_by_selector_obj(&selector)
}

pub fn find_window_by_selector_obj(
    selector: &selector::Selector,
) -> Result<Window, AutomationError> {
    if selector.path.is_empty() {
        return Err(AutomationError::Other("Selector has no paths".to_string()));
    }

    let first_path = &selector.path[0];
    if first_path.element_type != "Window" {
        return Err(AutomationError::Other(
            "First selector path must be Window".to_string(),
        ));
    }

    let windows = find_windows()?;
    let mut matches = windows
        .into_iter()
        .filter(|w| {
            selector::window_matches_criteria(&w.title, &w.class_name, &first_path.criteria)
        })
        .collect::<Vec<_>>();

    if matches.is_empty() {
        return Err(AutomationError::WindowNotFound {
            title: format!("No window matches selector: {}", selector.original),
        });
    }

    if matches.len() > 1 {
        log::warn!(
            "Selector '{}' matched {} windows, returning first. Windows: {:?}",
            selector.original,
            matches.len(),
            matches.iter().map(|w| &w.title).collect::<Vec<_>>()
        );
    }

    Ok(matches.remove(0))
}

pub fn find_control_by_selector(dsl: &str) -> Result<Control, AutomationError> {
    let selector = selector::Selector::parse(dsl)?;
    find_control_by_selector_obj(&selector)
}

pub fn find_control_by_selector_obj(
    selector: &selector::Selector,
) -> Result<Control, AutomationError> {
    if selector.path.is_empty() {
        return Err(AutomationError::Other("Selector has no paths".to_string()));
    }

    // First path must be Window
    let first_path = &selector.path[0];
    if first_path.element_type != "Window" {
        return Err(AutomationError::Other(
            "First selector path must be Window".to_string(),
        ));
    }

    // Find the window
    let mut windows = find_windows()?;
    windows.retain(|w| {
        selector::window_matches_criteria(&w.title, &w.class_name, &first_path.criteria)
    });

    if windows.is_empty() {
        return Err(AutomationError::WindowNotFound {
            title: format!("No window matches selector: {}", selector.original),
        });
    }

    if windows.len() > 1 {
        log::warn!(
            "Selector window matched {} windows, using first. Windows: {:?}",
            windows.len(),
            windows.iter().map(|w| &w.title).collect::<Vec<_>>()
        );
    }

    let window = windows.remove(0);
    let mut current_parent_hwnd = window.id.as_hwnd();

    // Process remaining control paths
    for (path_idx, path) in selector.path.iter().enumerate().skip(1) {
        if path.element_type != "Control" {
            return Err(AutomationError::Other(format!(
                "Selector path {} must be Control, got {}",
                path_idx, path.element_type
            )));
        }

        let controls = find_controls_in_window(current_parent_hwnd)?;
        let mut matches = controls
            .into_iter()
            .filter(|c| selector::control_matches_criteria(&c.text, &c.class_name, &path.criteria))
            .collect::<Vec<_>>();

        if matches.is_empty() {
            return Err(AutomationError::WindowNotFound {
                title: format!(
                    "No control matches selector at path {}: {}",
                    path_idx, selector.original
                ),
            });
        }

        if matches.len() > 1 {
            log::warn!(
                "Selector path {} matched {} controls, returning first. Controls: {:?}",
                path_idx,
                matches.len(),
                matches.iter().map(|c| &c.text).collect::<Vec<_>>()
            );
        }

        let control = matches.remove(0);
        current_parent_hwnd = control.id.as_hwnd();

        // If this is the last path, return the control
        if path_idx == selector.path.len() - 1 {
            return Ok(control);
        }
    }

    Err(AutomationError::Other(
        "Selector resolution failed unexpectedly".to_string(),
    ))
}

/// Generate a selector DSL string from a Window
pub fn window_to_selector(window: &Window) -> Result<String, AutomationError> {
    generator::window_to_selector(window)
}

/// Generate a selector DSL string from a Control and its parent Window
pub fn control_to_selector(
    control: &Control,
    parent_window: &Window,
) -> Result<String, AutomationError> {
    generator::control_to_selector(control, parent_window)
}

pub fn click(x: i32, y: i32) -> Result<(), AutomationError> {
    unsafe {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);

        sleep(Duration::from_millis(50));

        input.Anonymous.mi.dwFlags = MOUSEEVENTF_LEFTUP;
        SendInput(&[input], size_of::<INPUT>() as i32);

        Ok(())
    }
}

pub fn right_click(x: i32, y: i32) -> Result<(), AutomationError> {
    unsafe {
        let mut input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_RIGHTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);

        sleep(Duration::from_millis(50));

        input.Anonymous.mi.dwFlags = MOUSEEVENTF_RIGHTUP;
        SendInput(&[input], size_of::<INPUT>() as i32);

        Ok(())
    }
}

pub fn double_click(x: i32, y: i32) -> Result<(), AutomationError> {
    click(x, y)?;
    sleep(Duration::from_millis(100));
    click(x, y)?;
    Ok(())
}

pub fn move_mouse(x: i32, y: i32) -> Result<(), AutomationError> {
    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: x,
                    dy: y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_MOVE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);

        Ok(())
    }
}

pub fn type_text(text: &str) -> Result<(), AutomationError> {
    for ch in text.chars() {
        if ch == '\n' {
            press_key_code(0x0D)?;
        } else if ch == '\t' {
            press_key_code(0x09)?;
        } else {
            type_char(ch)?;
        }
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

fn type_char(ch: char) -> Result<(), AutomationError> {
    unsafe {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: ch as u16,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);

        Ok(())
    }
}

pub fn key_down(key_code: u16) -> Result<(), AutomationError> {
    unsafe {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(key_code),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);
        Ok(())
    }
}

pub fn key_up(key_code: u16) -> Result<(), AutomationError> {
    unsafe {
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(key_code),
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);
        Ok(())
    }
}

pub fn press_key_code(key_code: u16) -> Result<(), AutomationError> {
    key_down(key_code)?;
    sleep(Duration::from_millis(50));
    key_up(key_code)?;
    Ok(())
}

pub fn press_key(key: char) -> Result<(), AutomationError> {
    let key_code = match key.to_uppercase().next().unwrap() {
        'A' => 0x41,
        'B' => 0x42,
        'C' => 0x43,
        'D' => 0x44,
        'E' => 0x45,
        'F' => 0x46,
        'G' => 0x47,
        'H' => 0x48,
        'I' => 0x49,
        'J' => 0x4A,
        'K' => 0x4B,
        'L' => 0x4C,
        'M' => 0x4D,
        'N' => 0x4E,
        'O' => 0x4F,
        'P' => 0x50,
        'Q' => 0x51,
        'R' => 0x52,
        'S' => 0x53,
        'T' => 0x54,
        'U' => 0x55,
        'V' => 0x56,
        'W' => 0x57,
        'X' => 0x58,
        'Y' => 0x59,
        'Z' => 0x5A,
        '0' => 0x30,
        '1' => 0x31,
        '2' => 0x32,
        '3' => 0x33,
        '4' => 0x34,
        '5' => 0x35,
        '6' => 0x36,
        '7' => 0x37,
        '8' => 0x38,
        '9' => 0x39,
        _ => return Err(AutomationError::Other(format!("Unsupported key: {}", key))),
    };
    press_key_code(key_code)
}

pub fn press_enter() -> Result<(), AutomationError> {
    press_key_code(0x0D)
}

pub fn press_escape() -> Result<(), AutomationError> {
    press_key_code(0x1B)
}

pub fn press_tab() -> Result<(), AutomationError> {
    press_key_code(0x09)
}

pub fn key_combination(modifier: &str, key_code: u16) -> Result<(), AutomationError> {
    unsafe {
        let modifier_code = match modifier.to_uppercase().as_str() {
            "CTRL" | "CONTROL" => 0xA2,
            "SHIFT" => 0xA0,
            "ALT" => 0xA4,
            _ => {
                return Err(AutomationError::Other(format!(
                    "Unsupported modifier: {}",
                    modifier
                )));
            }
        };

        let mut input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(modifier_code),
                    wScan: 0,
                    dwFlags: KEYBD_EVENT_FLAGS(0),
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);

        sleep(Duration::from_millis(50));

        input.Anonymous.ki.wVk = VIRTUAL_KEY(key_code);
        SendInput(&[input], size_of::<INPUT>() as i32);

        sleep(Duration::from_millis(50));

        input.Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        SendInput(&[input], size_of::<INPUT>() as i32);

        sleep(Duration::from_millis(50));

        input.Anonymous.ki.wVk = VIRTUAL_KEY(modifier_code);
        SendInput(&[input], size_of::<INPUT>() as i32);

        Ok(())
    }
}

pub fn press_backspace() -> Result<(), AutomationError> {
    press_key_code(VK_BACKSPACE)
}

pub fn press_delete() -> Result<(), AutomationError> {
    press_key_code(VK_DELETE)
}

pub fn press_home() -> Result<(), AutomationError> {
    press_key_code(VK_HOME)
}

pub fn press_end() -> Result<(), AutomationError> {
    press_key_code(VK_END)
}

pub fn press_page_up() -> Result<(), AutomationError> {
    press_key_code(VK_PAGE_UP)
}

pub fn press_page_down() -> Result<(), AutomationError> {
    press_key_code(VK_PAGE_DOWN)
}

pub fn press_arrow_up() -> Result<(), AutomationError> {
    press_key_code(VK_UP)
}

pub fn press_arrow_down() -> Result<(), AutomationError> {
    press_key_code(VK_DOWN)
}

pub fn press_arrow_left() -> Result<(), AutomationError> {
    press_key_code(VK_LEFT)
}

pub fn press_arrow_right() -> Result<(), AutomationError> {
    press_key_code(VK_RIGHT)
}

pub fn press_insert() -> Result<(), AutomationError> {
    press_key_code(VK_INSERT)
}

pub fn press_f_key(n: u8) -> Result<(), AutomationError> {
    if n < 1 || n > 12 {
        return Err(AutomationError::Other(format!(
            "F key must be F1-F12, got F{}",
            n
        )));
    }
    let key_code = VK_F1 + (n - 1) as u16;
    press_key_code(key_code)
}

pub fn press_key_by_name(name: &str) -> Result<(), AutomationError> {
    let name_lower = name.to_lowercase();

    let key_code = match name_lower.as_str() {
        "backspace" => VK_BACKSPACE,
        "tab" => VK_TAB,
        "enter" | "return" => VK_RETURN,
        "escape" | "esc" => VK_ESCAPE,
        "space" => VK_SPACE,
        "delete" | "del" => VK_DELETE,
        "insert" | "ins" => VK_INSERT,
        "home" => VK_HOME,
        "end" => VK_END,
        "pageup" | "page_up" => VK_PAGE_UP,
        "pagedown" | "page_down" => VK_PAGE_DOWN,
        "arrowup" | "arrow_up" | "up" => VK_UP,
        "arrowdown" | "arrow_down" | "down" => VK_DOWN,
        "arrowleft" | "arrow_left" | "left" => VK_LEFT,
        "arrowright" | "arrow_right" | "right" => VK_RIGHT,
        "shift" => VK_SHIFT,
        "control" | "ctrl" => VK_CONTROL,
        "alt" => VK_ALT,
        _ if name_lower.starts_with('f') && name_lower.len() <= 3 => {
            if let Ok(n) = name_lower[1..].parse::<u8>() {
                if n >= 1 && n <= 12 {
                    VK_F1 + (n - 1) as u16
                } else {
                    return Err(AutomationError::Other(format!(
                        "F key must be F1-F12, got {}",
                        name
                    )));
                }
            } else {
                return Err(AutomationError::Other(format!("Invalid F key: {}", name)));
            }
        }
        _ => {
            return Err(AutomationError::Other(format!(
                "Unknown key name: {}",
                name
            )));
        }
    };

    press_key_code(key_code)
}

pub fn key_down_shift() -> Result<(), AutomationError> {
    key_down(0xA0)
}

pub fn key_up_shift() -> Result<(), AutomationError> {
    key_up(0xA0)
}

pub fn key_down_ctrl() -> Result<(), AutomationError> {
    key_down(0xA2)
}

pub fn key_up_ctrl() -> Result<(), AutomationError> {
    key_up(0xA2)
}

pub fn key_down_alt() -> Result<(), AutomationError> {
    key_down(0xA4)
}

pub fn key_up_alt() -> Result<(), AutomationError> {
    key_up(0xA4)
}

fn poll_until<F, T>(
    timeout_ms: u32,
    poll_interval_ms: u32,
    mut predicate: F,
) -> Result<T, AutomationError>
where
    F: FnMut() -> Option<T>,
{
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms as u64);
    let poll_interval = Duration::from_millis(poll_interval_ms as u64);

    loop {
        if let Some(result) = predicate() {
            return Ok(result);
        }

        if start.elapsed() >= timeout {
            return Err(AutomationError::Other(format!(
                "Timeout after {}ms waiting for condition",
                timeout_ms
            )));
        }

        sleep(poll_interval);
    }
}

pub fn wait_for_window(
    title_contains: &str,
    timeout_ms: u32,
    poll_interval_ms: u32,
) -> Result<Window, AutomationError> {
    poll_until(timeout_ms, poll_interval_ms, || {
        find_windows_by_title(title_contains)
            .ok()
            .and_then(|windows| windows.into_iter().next())
    })
}

pub fn wait_for_window_by_class(
    class_name: &str,
    timeout_ms: u32,
    poll_interval_ms: u32,
) -> Result<Window, AutomationError> {
    poll_until(timeout_ms, poll_interval_ms, || {
        find_windows_by_class(class_name)
            .ok()
            .and_then(|windows| windows.into_iter().next())
    })
}

pub fn wait_for_control(
    parent_hwnd: HWND,
    class_name: &str,
    timeout_ms: u32,
    poll_interval_ms: u32,
) -> Result<Control, AutomationError> {
    poll_until(timeout_ms, poll_interval_ms, || {
        find_controls_by_class(parent_hwnd, class_name)
            .ok()
            .and_then(|controls| controls.into_iter().next())
    })
}

pub fn wait_for_control_text(
    parent_hwnd: HWND,
    text: &str,
    timeout_ms: u32,
    poll_interval_ms: u32,
) -> Result<Control, AutomationError> {
    poll_until(timeout_ms, poll_interval_ms, || {
        find_controls_in_window(parent_hwnd)
            .ok()
            .and_then(|controls| {
                controls
                    .into_iter()
                    .find(|c| c.text.to_lowercase().contains(&text.to_lowercase()))
            })
    })
}

pub fn key_sequence(sequence: &str) -> Result<(), AutomationError> {
    let parts: Vec<&str> = sequence.split('+').map(|s| s.trim()).collect();

    if parts.is_empty() {
        return Err(AutomationError::Other("Empty key sequence".to_string()));
    }

    let mut modifiers = Vec::new();
    let mut final_key = "";

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            final_key = part;
        } else {
            modifiers.push(part.to_lowercase());
        }
    }

    for modifier in &modifiers {
        match modifier.as_str() {
            "ctrl" | "control" => key_down_ctrl()?,
            "shift" => key_down_shift()?,
            "alt" => key_down_alt()?,
            _ => {
                return Err(AutomationError::Other(format!(
                    "Unknown modifier: {}",
                    modifier
                )));
            }
        }
        sleep(Duration::from_millis(50));
    }

    press_key_by_name(final_key)?;

    for modifier in modifiers.iter().rev() {
        sleep(Duration::from_millis(50));
        match modifier.as_str() {
            "ctrl" | "control" => key_up_ctrl()?,
            "shift" => key_up_shift()?,
            "alt" => key_up_alt()?,
            _ => {}
        }
    }

    Ok(())
}

fn get_current_mouse_position() -> Result<(i32, i32), AutomationError> {
    unsafe {
        let mut point = windows::Win32::Foundation::POINT::default();
        GetCursorPos(&mut point).map_err(|e| AutomationError::Win32Failure { code: e.code().0 })?;
        Ok((point.x, point.y))
    }
}

pub fn scroll_wheel_at(
    x: i32,
    y: i32,
    direction: &str,
    amount: i32,
) -> Result<(), AutomationError> {
    if amount <= 0 {
        return Err(AutomationError::Other(
            "Scroll amount must be greater than 0".to_string(),
        ));
    }

    let dir_lower = direction.to_lowercase();
    let delta = match dir_lower.as_str() {
        "up" => WHEEL_DELTA * amount,
        "down" => -(WHEEL_DELTA * amount),
        _ => {
            return Err(AutomationError::Other(format!(
                "Invalid scroll direction: {}. Use 'up' or 'down'",
                direction
            )));
        }
    };

    move_mouse(x, y)?;
    sleep(Duration::from_millis(50));

    for _ in 0..amount {
        unsafe {
            let input = INPUT {
                r#type: INPUT_MOUSE,
                Anonymous: INPUT_0 {
                    mi: MOUSEINPUT {
                        dx: x,
                        dy: y,
                        mouseData: delta as u32,
                        dwFlags: MOUSEEVENTF_WHEEL,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            SendInput(&[input], size_of::<INPUT>() as i32);
        }
        sleep(Duration::from_millis(50));
    }

    Ok(())
}

pub fn scroll_in_window(
    parent_hwnd: HWND,
    direction: &str,
    amount: i32,
) -> Result<(), AutomationError> {
    if amount <= 0 {
        return Err(AutomationError::Other(
            "Scroll amount must be greater than 0".to_string(),
        ));
    }

    unsafe {
        let _ = SetForegroundWindow(parent_hwnd);
    }
    sleep(Duration::from_millis(50));

    let sb_direction = match direction.to_lowercase().as_str() {
        "up" => SB_LINEUP,
        "down" => SB_LINEDOWN,
        "left" => SB_LINELEFT,
        "right" => SB_LINERIGHT,
        _ => {
            return Err(AutomationError::Other(format!(
                "Invalid scroll direction: {}. Use 'up', 'down', 'left', or 'right'",
                direction
            )));
        }
    };

    for _ in 0..amount {
        unsafe {
            SendMessageW(parent_hwnd, WM_SCROLL, Some(WPARAM(sb_direction)), None);
        }
        sleep(Duration::from_millis(50));
    }

    Ok(())
}

pub fn scroll_up() -> Result<(), AutomationError> {
    let (x, y) = get_current_mouse_position()?;
    scroll_wheel_at(x, y, "up", 1)
}

pub fn scroll_down() -> Result<(), AutomationError> {
    let (x, y) = get_current_mouse_position()?;
    scroll_wheel_at(x, y, "down", 1)
}

pub fn scroll_page_up(hwnd: HWND) -> Result<(), AutomationError> {
    scroll_in_window(hwnd, "up", 5)
}

pub fn scroll_page_down(hwnd: HWND) -> Result<(), AutomationError> {
    scroll_in_window(hwnd, "down", 5)
}

fn linspace(from: (i32, i32), to: (i32, i32), steps: u32) -> Vec<(f64, f64)> {
    if steps == 0 {
        return vec![(from.0 as f64, from.1 as f64)];
    }

    (0..=steps)
        .map(|i| {
            let t = i as f64 / steps as f64;
            (
                from.0 as f64 + (to.0 - from.0) as f64 * t,
                from.1 as f64 + (to.1 - from.1) as f64 * t,
            )
        })
        .collect()
}

pub fn drag_mouse(
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    duration_ms: u32,
) -> Result<(), AutomationError> {
    if duration_ms < 50 {
        return Err(AutomationError::Other(
            "Drag duration must be at least 50ms".to_string(),
        ));
    }

    move_mouse(from_x, from_y)?;
    sleep(Duration::from_millis(50));

    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: from_x,
                    dy: from_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);
    }
    sleep(Duration::from_millis(50));

    let step_count = (duration_ms / 50).max(1);
    let steps = linspace((from_x, from_y), (to_x, to_y), step_count);

    for (x, y) in steps {
        move_mouse(x as i32, y as i32)?;
        sleep(Duration::from_millis(50));
    }

    unsafe {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: to_x,
                    dy: to_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };

        SendInput(&[input], size_of::<INPUT>() as i32);
    }

    Ok(())
}

pub fn drag_control(
    source: &Control,
    to_x: i32,
    to_y: i32,
    duration_ms: u32,
) -> Result<(), AutomationError> {
    let from_x = source.bounds.left + source.bounds.width / 2;
    let from_y = source.bounds.top + source.bounds.height / 2;
    drag_mouse(from_x, from_y, to_x, to_y, duration_ms)
}
