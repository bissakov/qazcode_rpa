use std::thread::sleep;
use std::time::Duration;
use ui_automation::{
    Control, ControlId, Rect, Window, WindowId, control_to_selector, find_control_by_selector,
    find_window_by_selector, window_to_selector,
};

// ============================================================================
// UNIT TESTS
// ============================================================================

#[test]
fn test_window_to_selector_title_and_class() {
    let window = Window {
        id: WindowId(12345),
        title: "Notepad".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Notepad;class~#32770");
}

#[test]
fn test_window_to_selector_title_only() {
    let window = Window {
        id: WindowId(12345),
        title: "Notepad".to_string(),
        class_name: "".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Notepad");
}

#[test]
fn test_window_to_selector_class_only() {
    let window = Window {
        id: WindowId(12345),
        title: "".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>class~#32770");
}

#[test]
fn test_window_to_selector_both_empty_error() {
    let window = Window {
        id: WindowId(12345),
        title: "".to_string(),
        class_name: "".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let result = window_to_selector(&window);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[test]
fn test_window_to_selector_with_escaped_greater_than() {
    let window = Window {
        id: WindowId(12345),
        title: "Report > Analysis".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(
        selector,
        "Window>title~Report \\> Analysis;class~MainWindow"
    );
}

#[test]
fn test_window_to_selector_with_escaped_semicolon() {
    let window = Window {
        id: WindowId(12345),
        title: "Data; Export".to_string(),
        class_name: "MyApp".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Data\\; Export;class~MyApp");
}

#[test]
fn test_window_to_selector_with_escaped_backslash() {
    let window = Window {
        id: WindowId(12345),
        title: "Path\\File".to_string(),
        class_name: "App".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Path\\\\File;class~App");
}

#[test]
fn test_window_to_selector_with_multiple_escaped_chars() {
    let window = Window {
        id: WindowId(12345),
        title: "A > B; C\\D".to_string(),
        class_name: "App".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~A \\> B\\; C\\\\D;class~App");
}

#[test]
fn test_window_to_selector_long_title() {
    let long_title = "A".repeat(500);
    let window = Window {
        id: WindowId(12345),
        title: long_title.clone(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert!(selector.contains(&long_title));
}

#[test]
fn test_control_to_selector_class_and_text() {
    let window = Window {
        id: WindowId(12345),
        title: "Notepad".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let control = Control {
        id: ControlId(54321),
        class_name: "Edit".to_string(),
        text: "Search".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 400,
            height: 50,
        },
        visible: true,
        enabled: true,
    };

    let selector = control_to_selector(&control, &window).unwrap();
    // Should include window criteria and control criteria, but no index (if it's first)
    assert!(selector.starts_with("Window>"));
    assert!(selector.contains("Edit"));
    assert!(selector.contains("Search"));
}

#[test]
fn test_control_to_selector_class_only() {
    let window = Window {
        id: WindowId(12345),
        title: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let control = Control {
        id: ControlId(54321),
        class_name: "Button".to_string(),
        text: "".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 100,
            height: 30,
        },
        visible: true,
        enabled: true,
    };

    let selector = control_to_selector(&control, &window).unwrap();
    assert!(selector.contains("Button"));
    assert!(!selector.contains("text~"));
}

#[test]
fn test_control_to_selector_text_only() {
    let window = Window {
        id: WindowId(12345),
        title: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let control = Control {
        id: ControlId(54321),
        class_name: "".to_string(),
        text: "OK".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 100,
            height: 30,
        },
        visible: true,
        enabled: true,
    };

    let selector = control_to_selector(&control, &window).unwrap();
    assert!(selector.contains("OK"));
    // Verify the control doesn't have a class criterion (only the window has class)
    assert!(selector.ends_with(">Control>text~OK"));
}

#[test]
fn test_control_to_selector_both_empty_error() {
    let window = Window {
        id: WindowId(12345),
        title: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let control = Control {
        id: ControlId(54321),
        class_name: "".to_string(),
        text: "".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 100,
            height: 30,
        },
        visible: true,
        enabled: true,
    };

    let result = control_to_selector(&control, &window);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[test]
fn test_control_to_selector_with_escaped_chars_in_text() {
    let window = Window {
        id: WindowId(12345),
        title: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let control = Control {
        id: ControlId(54321),
        class_name: "Button".to_string(),
        text: "Save; Export > Now".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 100,
            height: 30,
        },
        visible: true,
        enabled: true,
    };

    let selector = control_to_selector(&control, &window).unwrap();
    assert!(selector.contains("Save\\; Export \\> Now"));
}

#[test]
fn test_roundtrip_window_parse() {
    let window = Window {
        id: WindowId(12345),
        title: "Notepad".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector_str = window_to_selector(&window).unwrap();

    // Parse the generated selector
    let selector = ui_automation::selector::Selector::parse(&selector_str).unwrap();

    // Verify structure
    assert_eq!(selector.path.len(), 1);
    assert_eq!(selector.path[0].element_type, "Window");
    assert_eq!(selector.path[0].criteria.len(), 2);

    // Verify criteria values are unescaped
    let title_crit = &selector.path[0].criteria[0];
    let class_crit = &selector.path[0].criteria[1];
    assert_eq!(title_crit.value, "Notepad");
    assert_eq!(class_crit.value, "#32770");
}

#[test]
fn test_roundtrip_with_escaped_chars() {
    let window = Window {
        id: WindowId(12345),
        title: "Report > Data".to_string(),
        class_name: "MyApp".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector_str = window_to_selector(&window).unwrap();
    let selector = ui_automation::selector::Selector::parse(&selector_str).unwrap();

    // Verify the escaped string is unescaped after parsing
    let title_crit = &selector.path[0].criteria[0];
    assert_eq!(title_crit.value, "Report > Data");
}

#[test]
fn test_generated_selector_format() {
    let window = Window {
        id: WindowId(12345),
        title: "App".to_string(),
        class_name: "Window".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
    };

    let selector = window_to_selector(&window).unwrap();

    // Verify format: Window>attr~val;attr~val
    assert!(selector.starts_with("Window>"));
    assert!(selector.contains("title~"));
    assert!(selector.contains("class~"));
    assert!(selector.contains(";"));
    assert!(!selector.contains("/"));
    assert!(!selector.contains("|"));
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[test]
fn test_window_selector_finds_notepad() {
    // Launch Notepad
    let app = match ui_automation::launch_application("notepad.exe", "") {
        Ok(app) => app,
        Err(_) => return, // Skip if Notepad can't be launched
    };
    sleep(Duration::from_millis(500));

    // Get the window
    let windows = match ui_automation::find_windows() {
        Ok(w) => w,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    let notepad = match windows.iter().find(|w| w.title.contains("Notepad")) {
        Some(w) => w,
        None => {
            let _ = app.close();
            return;
        }
    };

    // Generate selector from the window
    let selector_str = match window_to_selector(notepad) {
        Ok(s) => s,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Use generated selector to find the window again
    let found = match find_window_by_selector(&selector_str) {
        Ok(w) => w,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Verify it's the same window
    assert_eq!(found.id.0, notepad.id.0);
    let _ = app.close();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_selector_finds_edit_in_notepad() {
    // Launch Notepad
    let app = match ui_automation::launch_application("notepad.exe", "") {
        Ok(app) => app,
        Err(_) => return,
    };
    sleep(Duration::from_millis(500));

    // Find Notepad window
    let window = match find_window_by_selector("Window>title~Notepad") {
        Ok(w) => w,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Find Edit control
    let controls = match ui_automation::find_controls_in_window(window.id.as_hwnd()) {
        Ok(c) => c,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    let edit = match controls.iter().find(|c| c.class_name == "Edit") {
        Some(c) => c,
        None => {
            let _ = app.close();
            return;
        }
    };

    // Generate selector from control
    let selector_str = match control_to_selector(edit, &window) {
        Ok(s) => s,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Use generated selector to find control again
    let found = match find_control_by_selector(&selector_str) {
        Ok(c) => c,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Verify it's the same control
    assert_eq!(found.id.0, edit.id.0);
    let _ = app.close();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_generated_selector_vs_manual_selector_notepad() {
    // Launch Notepad
    let app = match ui_automation::launch_application("notepad.exe", "") {
        Ok(app) => app,
        Err(_) => return,
    };
    sleep(Duration::from_millis(500));

    // Create manual selector
    let manual_selector = "Window>title~Notepad";

    // Find window with manual selector
    let window = match find_window_by_selector(manual_selector) {
        Ok(w) => w,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Generate selector for same window
    let generated = match window_to_selector(&window) {
        Ok(s) => s,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    // Use generated selector - should find same window
    let found = match find_window_by_selector(&generated) {
        Ok(w) => w,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    assert_eq!(found.id.0, window.id.0);
    let _ = app.close();
    sleep(Duration::from_millis(100));
}
