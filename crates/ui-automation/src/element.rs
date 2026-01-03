use crate::error::{from_error_code, Error, Result};
use crate::types::Rect;
use std::ffi::CString;
use ui_automation_sys as sys;

pub struct Element {
    handle: *mut sys::ElementHandle,
}

impl Element {
    pub fn find_by_name(name: &str) -> Option<Self> {
        Self::find_by_name_with_timeout(name, 0)
    }

    pub fn find_by_name_with_timeout(name: &str, timeout_ms: i32) -> Option<Self> {
        let c_name = CString::new(name).ok()?;
        let handle = unsafe { sys::element_find_by_name(c_name.as_ptr(), timeout_ms) };

        if handle.is_null() {
            None
        } else {
            Some(Element { handle })
        }
    }

    pub fn find_by_automation_id(id: &str) -> Option<Self> {
        Self::find_by_automation_id_with_timeout(id, 0)
    }

    pub fn find_by_automation_id_with_timeout(id: &str, timeout_ms: i32) -> Option<Self> {
        let c_id = CString::new(id).ok()?;
        let handle = unsafe { sys::element_find_by_automation_id(c_id.as_ptr(), timeout_ms) };

        if handle.is_null() {
            None
        } else {
            Some(Element { handle })
        }
    }

    pub fn find_by_class_name(class_name: &str) -> Option<Self> {
        Self::find_by_class_name_with_timeout(class_name, 0)
    }

    pub fn find_by_class_name_with_timeout(class_name: &str, timeout_ms: i32) -> Option<Self> {
        let c_class = CString::new(class_name).ok()?;
        let handle = unsafe { sys::element_find_by_class_name(c_class.as_ptr(), timeout_ms) };

        if handle.is_null() {
            None
        } else {
            Some(Element { handle })
        }
    }

    pub fn get_children(&self) -> Vec<Element> {
        let mut count: i32 = 0;
        let children_ptr = unsafe { sys::element_get_children(self.handle, &mut count) };

        if children_ptr.is_null() || count == 0 {
            return Vec::new();
        }

        let mut children = Vec::new();
        for i in 0..count {
            unsafe {
                let child_handle = *children_ptr.offset(i as isize);
                if !child_handle.is_null() {
                    children.push(Element { handle: child_handle });
                }
            }
        }

        unsafe {
            libc::free(children_ptr as *mut libc::c_void);
        }

        children
    }

    pub fn get_parent(&self) -> Option<Element> {
        let handle = unsafe { sys::element_get_parent(self.handle) };

        if handle.is_null() {
            None
        } else {
            Some(Element { handle })
        }
    }

    pub fn get_text(&self) -> Result<String> {
        let mut buffer = vec![0u8; 1024];
        let result = unsafe {
            sys::element_get_text(
                self.handle,
                buffer.as_mut_ptr() as *mut i8,
                buffer.len() as i32,
            )
        };

        if result == sys::SUCCESS {
            let null_pos = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            String::from_utf8(buffer[..null_pos].to_vec())
                .map_err(|_| Error::OperationFailed("Invalid UTF-8 string".to_string()))
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn set_text(&self, text: &str) -> Result<()> {
        let c_text = CString::new(text)
            .map_err(|_| Error::OperationFailed("Invalid text string".to_string()))?;
        let result = unsafe { sys::element_set_text(self.handle, c_text.as_ptr()) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn click(&self) -> Result<()> {
        let result = unsafe { sys::element_click(self.handle) };

        if result == sys::SUCCESS {
            Ok(())
        } else {
            Err(from_error_code(result))
        }
    }

    pub fn invoke(&self) -> Result<()> {
        let result = unsafe { sys::element_invoke(self.handle) };

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

        let result = unsafe { sys::element_get_rect(self.handle, &mut rect) };

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

    pub fn is_enabled(&self) -> bool {
        let result = unsafe { sys::element_is_enabled(self.handle) };
        result != 0
    }
}

impl Drop for Element {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::element_free(self.handle) };
        }
    }
}

unsafe impl Send for Element {}
unsafe impl Sync for Element {}
