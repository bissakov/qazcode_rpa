#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    String(String),
    Int(i32),
    Bool(bool),
    Double(f64),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    Value,
    Invoke,
    Selection,
    Text,
    Window,
}
