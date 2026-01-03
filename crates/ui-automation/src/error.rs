use std::fmt;
use ui_automation_sys as sys;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    WindowNotFound,
    ElementNotFound,
    InvalidHandle,
    OperationFailed(String),
    Timeout,
    NullPointer,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WindowNotFound => write!(f, "Window not found"),
            Error::ElementNotFound => write!(f, "Element not found"),
            Error::InvalidHandle => write!(f, "Invalid handle"),
            Error::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
            Error::Timeout => write!(f, "Operation timed out"),
            Error::NullPointer => write!(f, "Null pointer"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn from_error_code(code: i32) -> Error {
    match code {
        sys::ERROR_WINDOW_NOT_FOUND => Error::WindowNotFound,
        sys::ERROR_ELEMENT_NOT_FOUND => Error::ElementNotFound,
        sys::ERROR_INVALID_HANDLE => Error::InvalidHandle,
        sys::ERROR_TIMEOUT => Error::Timeout,
        sys::ERROR_NULL_POINTER => Error::NullPointer,
        _ => Error::OperationFailed(format!("Error code: {}", code)),
    }
}
