use std::fmt::{self, Display, Formatter};

use super::selector::Selector;

#[derive(Debug)]
pub enum AutomationError {
    NotSupported(String),
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
            Self::NotSupported(msg) => {
                write!(f, "Not supported on Linux: {msg}")
            }
            Self::ApplicationNotFound { name } => {
                write!(f, "Application '{name}' not found in running processes")
            }
            Self::WindowNotFound { title } => {
                write!(f, "Window with title containing '{title}' not found")
            }
            Self::ProcessTerminated { pid } => {
                write!(f, "Process with PID {pid} has terminated")
            }
            Self::ProcessNotFound { name } => {
                write!(f, "Process '{name}' not found")
            }
            Self::AccessDenied { operation } => {
                write!(f, "Access denied for operation: {operation}")
            }
            Self::Other(msg) => {
                write!(f, "{msg}")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationId(pub u32);

pub struct Application {
    pid: u32,
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
        false
    }

    pub fn close(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Application.close".to_string(),
        ))
    }

    pub fn wait_for_exit(&self, _timeout_ms: Option<u32>) -> Result<u32, AutomationError> {
        Err(AutomationError::NotSupported(
            "Application.wait_for_exit".to_string(),
        ))
    }
}

pub fn launch_application(_exe: &str, _args: &str) -> Result<Application, AutomationError> {
    Err(AutomationError::NotSupported(
        "launch_application".to_string(),
    ))
}

pub fn show_overlay_on_rect(
    _rect: Rect,
    _color_rgb: (u8, u8, u8),
    _duration_ms: u32,
    _border_width: i32,
) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported(
        "show_overlay_on_rect".to_string(),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Window,
    Control,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElementId(pub isize);

impl ElementId {
    #[must_use]
    pub const fn as_hwnd(&self) -> ElementId {
        *self
    }
}

#[derive(Debug)]
pub struct Element {
    pub id: ElementId,
    pub element_type: ElementType,
    pub class_name: String,
    pub text: String,
    pub bounds: Rect,
    pub visible: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            left: 0,
            top: 0,
            width: 0,
            height: 0,
        }
    }
}

impl Element {
    fn require_window(&self, operation: &str) -> Result<(), AutomationError> {
        if self.element_type != ElementType::Window {
            return Err(AutomationError::Other(format!(
                "Cannot {} a control (only windows support this)",
                operation
            )));
        }
        Ok(())
    }

    fn require_control(&self, operation: &str) -> Result<(), AutomationError> {
        if self.element_type != ElementType::Control {
            return Err(AutomationError::Other(format!(
                "Cannot {} on a window (only controls support this)",
                operation
            )));
        }
        Ok(())
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn get_text_live(&mut self) -> Result<String, AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.get_text_live".to_string(),
        ))
    }

    pub fn set_text(&self, _text: &str) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.set_text".to_string(),
        ))
    }

    pub fn click(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported("Element.click".to_string()))
    }

    pub fn right_click(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.right_click".to_string(),
        ))
    }

    pub fn double_click(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.double_click".to_string(),
        ))
    }

    #[must_use]
    pub fn is_focused(&self) -> bool {
        false
    }

    pub fn focus(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported("Element.focus".to_string()))
    }

    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn refresh(&mut self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported("Element.refresh".to_string()))
    }

    #[must_use]
    pub fn get_process_id(&self) -> u32 {
        0
    }

    pub fn show_overlay(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.show_overlay".to_string(),
        ))
    }

    pub fn show_overlay_custom(
        &self,
        _color_rgb: (u8, u8, u8),
        _duration_ms: u32,
        _border_width: i32,
    ) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.show_overlay_custom".to_string(),
        ))
    }

    pub fn close(&self) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported("Element.close".to_string()))
    }

    pub fn activate(&self) -> Result<(), AutomationError> {
        self.require_window("activate")?;
        Err(AutomationError::NotSupported(
            "Element.activate".to_string(),
        ))
    }

    pub fn minimize(&self) -> Result<(), AutomationError> {
        self.require_window("minimize")?;
        Err(AutomationError::NotSupported(
            "Element.minimize".to_string(),
        ))
    }

    pub fn maximize(&self) -> Result<(), AutomationError> {
        self.require_window("maximize")?;
        Err(AutomationError::NotSupported(
            "Element.maximize".to_string(),
        ))
    }

    pub fn restore(&self) -> Result<(), AutomationError> {
        self.require_window("restore")?;
        Err(AutomationError::NotSupported("Element.restore".to_string()))
    }

    pub fn show(&self) -> Result<(), AutomationError> {
        self.require_window("show")?;
        Err(AutomationError::NotSupported("Element.show".to_string()))
    }

    pub fn resize(&self, _width: i32, _height: i32) -> Result<(), AutomationError> {
        self.require_window("resize")?;
        Err(AutomationError::NotSupported("Element.resize".to_string()))
    }

    pub fn move_to(&self, _x: i32, _y: i32) -> Result<(), AutomationError> {
        self.require_window("move_to")?;
        Err(AutomationError::NotSupported("Element.move_to".to_string()))
    }

    #[must_use]
    pub fn is_minimized(&self) -> bool {
        false
    }

    #[must_use]
    pub fn is_maximized(&self) -> bool {
        false
    }

    pub fn find_child_elements(&self) -> Result<Vec<Element>, AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.find_child_elements".to_string(),
        ))
    }

    pub fn check(&self) -> Result<(), AutomationError> {
        self.require_control("check")?;
        Err(AutomationError::NotSupported("Element.check".to_string()))
    }

    pub fn uncheck(&self) -> Result<(), AutomationError> {
        self.require_control("uncheck")?;
        Err(AutomationError::NotSupported("Element.uncheck".to_string()))
    }

    #[must_use]
    pub fn is_checked(&self) -> bool {
        false
    }

    pub fn select_text(&self, _start: usize, _end: usize) -> Result<(), AutomationError> {
        self.require_control("select_text")?;
        Err(AutomationError::NotSupported(
            "Element.select_text".to_string(),
        ))
    }

    pub fn select_all(&self) -> Result<(), AutomationError> {
        self.require_control("select_all")?;
        Err(AutomationError::NotSupported(
            "Element.select_all".to_string(),
        ))
    }

    pub fn get_selected_range(&self) -> Result<(usize, usize), AutomationError> {
        self.require_control("get_selected_range")?;
        Err(AutomationError::NotSupported(
            "Element.get_selected_range".to_string(),
        ))
    }

    pub fn scroll_to(&self, _x: i32, _y: i32) -> Result<(), AutomationError> {
        Err(AutomationError::NotSupported(
            "Element.scroll_to".to_string(),
        ))
    }
}

pub fn find_processes_by_name(_name: &str) -> Result<Vec<Application>, AutomationError> {
    Err(AutomationError::NotSupported(
        "find_processes_by_name".to_string(),
    ))
}

pub fn attach_to_process_by_pid(_pid: u32) -> Result<Application, AutomationError> {
    Err(AutomationError::NotSupported(
        "attach_to_process_by_pid".to_string(),
    ))
}

pub fn attach_to_process_by_name(_name: &str) -> Result<Application, AutomationError> {
    Err(AutomationError::NotSupported(
        "attach_to_process_by_name".to_string(),
    ))
}

pub fn find_windows() -> Result<Vec<Element>, AutomationError> {
    Err(AutomationError::NotSupported("find_windows".to_string()))
}

pub fn get_foreground_window() -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "get_foreground_window".to_string(),
    ))
}

pub fn find_child_elements(_parent_id: ElementId) -> Result<Vec<Element>, AutomationError> {
    Err(AutomationError::NotSupported(
        "find_child_elements".to_string(),
    ))
}

pub fn find_element_by_selector(_dsl: &str) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "find_element_by_selector".to_string(),
    ))
}

pub fn find_element_by_selector_obj(
    _selector: &Selector,
    _timeout_ms: Option<u32>,
) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "find_element_by_selector_obj".to_string(),
    ))
}

pub fn find_window_by_selector(_dsl: &str) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "find_window_by_selector".to_string(),
    ))
}

pub fn find_control_by_selector(_dsl: &str) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "find_control_by_selector".to_string(),
    ))
}

pub fn click(_x: i32, _y: i32) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("click".to_string()))
}

pub fn right_click(_x: i32, _y: i32) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("right_click".to_string()))
}

pub fn double_click(_x: i32, _y: i32) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("double_click".to_string()))
}

pub fn move_mouse(_x: i32, _y: i32) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("move_mouse".to_string()))
}

pub fn type_text(_text: &str) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("type_text".to_string()))
}

pub fn key_down(_key_code: u16) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("key_down".to_string()))
}

pub fn key_up(_key_code: u16) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("key_up".to_string()))
}

pub fn press_key_code(_key_code: u16) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("press_key_code".to_string()))
}

pub fn press_key(_key: char) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("press_key".to_string()))
}

pub fn key_combination(_modifier: &str, _key_code: u16) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("key_combination".to_string()))
}

pub fn press_f_key(_n: u8) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("press_f_key".to_string()))
}

pub fn press_key_by_name(_name: &str) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported(
        "press_key_by_name".to_string(),
    ))
}

pub fn wait_for_window(
    _dsl: &str,
    _timeout_ms: u32,
    _poll_interval_ms: u32,
) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported("wait_for_window".to_string()))
}

pub fn wait_for_control(
    _dsl: &str,
    _timeout_ms: u32,
    _poll_interval_ms: u32,
) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "wait_for_control".to_string(),
    ))
}

pub fn wait_for_control_text(
    _dsl: &str,
    _expected_text: &str,
    _timeout_ms: u32,
    _poll_interval_ms: u32,
) -> Result<Element, AutomationError> {
    Err(AutomationError::NotSupported(
        "wait_for_control_text".to_string(),
    ))
}

pub fn key_sequence(_sequence: &str) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("key_sequence".to_string()))
}

pub fn scroll_wheel_at(
    _x: i32,
    _y: i32,
    _delta: i32,
    _horizontal: bool,
) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("scroll_wheel_at".to_string()))
}

pub fn scroll_in_window(
    _element: &Element,
    _direction: &str,
    _times: usize,
) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported(
        "scroll_in_window".to_string(),
    ))
}

pub fn drag_mouse(
    _start_x: i32,
    _start_y: i32,
    _end_x: i32,
    _end_y: i32,
    _duration_ms: u32,
) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("drag_mouse".to_string()))
}

pub fn drag_control(
    _element: &Element,
    _target_x: i32,
    _target_y: i32,
    _duration_ms: u32,
) -> Result<(), AutomationError> {
    Err(AutomationError::NotSupported("drag_control".to_string()))
}
