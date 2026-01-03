#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::{c_char, c_int, c_void};

pub const SUCCESS: c_int = 0;
pub const ERROR_WINDOW_NOT_FOUND: c_int = -1;
pub const ERROR_ELEMENT_NOT_FOUND: c_int = -2;
pub const ERROR_INVALID_HANDLE: c_int = -3;
pub const ERROR_OPERATION_FAILED: c_int = -4;
pub const ERROR_TIMEOUT: c_int = -5;
pub const ERROR_NULL_POINTER: c_int = -6;

#[repr(C)]
pub struct Rect {
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
}

#[repr(C)]
pub struct WindowHandle {
    pub handle: *mut c_void,
}

#[repr(C)]
pub struct ElementHandle {
    pub handle: *mut c_void,
}

unsafe extern "C" {
    pub fn window_find_by_title(title: *const c_char) -> *mut WindowHandle;
    pub fn window_find_by_class(class_name: *const c_char) -> *mut WindowHandle;
    pub fn window_get_focused() -> *mut WindowHandle;
    pub fn window_get_all(count: *mut c_int) -> *mut *mut WindowHandle;
    pub fn window_get_rect(window: *const WindowHandle, rect: *mut Rect) -> c_int;
    pub fn window_is_visible(window: *const WindowHandle) -> c_int;
    pub fn window_set_focus(window: *const WindowHandle) -> c_int;
    pub fn window_close(window: *const WindowHandle) -> c_int;
    pub fn window_maximize(window: *const WindowHandle) -> c_int;
    pub fn window_minimize(window: *const WindowHandle) -> c_int;
    pub fn window_click(window: *const WindowHandle, x: c_int, y: c_int) -> c_int;
    pub fn window_double_click(window: *const WindowHandle, x: c_int, y: c_int) -> c_int;
    pub fn window_right_click(window: *const WindowHandle, x: c_int, y: c_int) -> c_int;
    pub fn window_type_text(window: *const WindowHandle, text: *const c_char) -> c_int;
    pub fn window_key_down(window: *const WindowHandle, key: c_int) -> c_int;
    pub fn window_key_up(window: *const WindowHandle, key: c_int) -> c_int;
    pub fn window_free(window: *mut WindowHandle);

    pub fn element_find_by_name(name: *const c_char, timeout_ms: c_int) -> *mut ElementHandle;
    pub fn element_find_by_automation_id(id: *const c_char, timeout_ms: c_int) -> *mut ElementHandle;
    pub fn element_find_by_class_name(class_name: *const c_char, timeout_ms: c_int) -> *mut ElementHandle;
    pub fn element_get_children(element: *const ElementHandle, count: *mut c_int) -> *mut *mut ElementHandle;
    pub fn element_get_parent(element: *const ElementHandle) -> *mut ElementHandle;
    pub fn element_get_text(element: *const ElementHandle, buffer: *mut c_char, buffer_size: c_int) -> c_int;
    pub fn element_set_text(element: *const ElementHandle, text: *const c_char) -> c_int;
    pub fn element_click(element: *const ElementHandle) -> c_int;
    pub fn element_invoke(element: *const ElementHandle) -> c_int;
    pub fn element_get_rect(element: *const ElementHandle, rect: *mut Rect) -> c_int;
    pub fn element_is_enabled(element: *const ElementHandle) -> c_int;
    pub fn element_free(element: *mut ElementHandle);

    pub fn init_uia() -> c_int;
    pub fn cleanup_uia();
}
