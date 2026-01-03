# UI Automation

Windows UI Automation library for Rust with a three-layer architecture:
- C library layer for Win32 and UIA APIs
- Safe Rust wrapper with idiomatic API
- Clean external interface

## Architecture

```
┌─────────────────────────────┐
│   ui-automation (Rust)      │  Safe, idiomatic API
│   Window, Element types     │
└──────────────┬──────────────┘
               │
┌──────────────▼──────────────┐
│   ui-automation-sys (FFI)   │  FFI bindings
│   Raw C function calls      │
└──────────────┬──────────────┘
               │
┌──────────────▼──────────────┐
│   C Library (CMake)         │  Win32 + UIA
│   window.c, element.c       │
└─────────────────────────────┘
```

## Usage

```rust
use ui_automation::{init, cleanup, Window, Element};

fn main() {
    init().expect("Failed to initialize UI Automation");

    // Window operations
    if let Some(window) = Window::find_by_title("Notepad") {
        window.set_focus().ok();
        window.type_text("Hello, World!").ok();

        let rect = window.get_rect().unwrap();
        println!("Window at {}x{}", rect.x, rect.y);
    }

    // Element operations
    if let Some(element) = Element::find_by_name("OK") {
        element.click().ok();
    }

    cleanup();
}
```

## Features

### Window Operations
- Find windows by title or class name
- Get focused window or all visible windows
- Click, type text, send keyboard events
- Window management (maximize, minimize, close)
- Get window position and size

### Element Operations
- Find elements by name, automation ID, or class name
- Navigate element tree (children, parent)
- Click elements or invoke buttons
- Get/set element text
- Get element properties and state

## Building

Requires:
- CMake 3.15+
- Windows SDK (for Win32 and UIA headers/libs)
- Rust toolchain

```bash
cargo build --release
```

## Examples

```bash
cargo run --example basic_usage
```

## Testing

```bash
cargo test
```
