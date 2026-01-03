use crate::error::{from_error_code, Error, Result};
use crate::types::Rect;
use std::ffi::CString;
use ui_automation_sys as sys;

pub struct Window {
    handle: *mut sys::WindowHandle,
}

impl Window {
    pub fn find_by_title(title: &str) -> Option<Self> {
        let c_title = CString::new(title).ok()?;
        let handle = unsafe { sys::window_find_by_title(c_title.as_ptr()) };

        if handle.is_null() {
            None
        } else {
            Some(Window { handle })
        }
    }

    pub fn find_by_class(class_name: &str) -> Option<Self> {
        let c_class = CString::new(class_name).ok()?;
        let handle = unsafe { sys::window_find_by_class(c_class.as_ptr()) };

        if handle.is_null() {
            None
        } else {
            Some(Window { handle })
        }
    }

    pub fn get_focused() -> Option<Self> {
        let handle = unsafe { sys::window_get_focused() };

        if handle.is_null() {
            None
        } else {
            Some(Window { handle })
        }
    }

    pub fn get_all_windows() -> Vec<Self> {
        let mut count: i32 = 0;
        let windows_ptr = unsafe { sys::window_get_all(&mut count) };

        if windows_ptr.is_null() || count == 0 {
            return Vec::new();
        }

        let mut windows = Vec::new();
        for i in 0..count {
            unsafe {
                let window_handle = *windows_ptr.offset(i as isize);
                if !window_handle.is_null() {
                    windows.push(Window { handle: window_handle });
                }
            }
        }

        unsafe {
            libc::free(windows_ptr as *mut libc::c_void);
        }

        windows
    }

    pub fn click(&self, x: i32, y: i32) -> Result<()> {
        let result = unsafe { sys::window_click(self.handle, x, y) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn double_click(&self, x: i32, y: i32) -> Result<()> {
        let result = unsafe { sys::window_double_click(self.handle, x, y) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn right_click(&self, x: i32, y: i32) -> Result<()> {
        let result = unsafe { sys::window_right_click(self.handle, x, y) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn type_text(&self, text: &str) -> Result<()> {
        let c_text = CString::new(text)
            .map_err(|_| Error::OperationFailed("Invalid text string".to_string()))?;
        let result = unsafe { sys::window_type_text(self.handle, c_text.as_ptr()) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn key_down(&self, key: i32) -> Result<()> {
        let result = unsafe { sys::window_key_down(self.handle, key) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn key_up(&self, key: i32) -> Result<()> {
        let result = unsafe { sys::window_key_up(self.handle, key) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn get_rect(&self) -> Result<Rect> {
        let mut rect = sys::Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        let result = unsafe { sys::window_get_rect(self.handle, &mut rect) };

        if result == sys::SUCCESS {
            Ok(Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            })
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn set_focus(&self) -> Result<()> {
        let result = unsafe { sys::window_set_focus(self.handle) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn is_visible(&self) -> bool {
        let result = unsafe { sys::window_is_visible(self.handle) };
        result != 0
    }

    pub fn close(&self) -> Result<()> {
        let result = unsafe { sys::window_close(self.handle) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn maximize(&self) -> Result<()> {
        let result = unsafe { sys::window_maximize(self.handle) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn minimize(&self) -> Result<()> {
        let result = unsafe { sys::window_minimize(self.handle) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::window_free(self.handle) };
        }
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}
