mod error;
mod types;
mod window;
mod element;

pub use error::{Error, Result};
pub use types::{Rect, PropertyValue, PatternType};
pub use window::Window;
pub use element::Element;

use ui_automation_sys as sys;

pub fn init() -> Result<()> {
    let result = unsafe { sys::init_uia() };
    if result == sys::SUCCESS {
        Ok(())
    } else {
        Err(Error::OperationFailed("Failed to initialize UI Automation".to_string()))
    }
}

pub fn cleanup() {
    unsafe { sys::cleanup_uia() };
}
