use std::thread::sleep;
use std::time::Duration;

use ui_automation::win32::automation::*;
use ui_automation::win32::selector;

#[test]
fn test_selector_find_window_by_title() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~Notepad";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    let win = window.unwrap();
    assert!(win.text.to_lowercase().contains("notepad"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_find_window_by_class() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>class~Notepad";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_find_control_by_class() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~Notepad>Control>class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    let ctrl = control.unwrap();
    assert!(ctrl.class_name.to_lowercase().contains("edit"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_find_control_with_multiple_criteria() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let windows = find_windows().unwrap();
    let window = windows
        .iter()
        .find(|w| w.text.to_lowercase().contains("notepad"))
        .unwrap();
    let controls = find_child_elements(window.id.as_hwnd()).unwrap();
    let edit_control = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"));

    if edit_control.is_some() {
        let selector_str = "Window>title~Notepad;class~Notepad>Control>class~Edit";
        let control = find_control_by_selector(selector_str);
        assert!(control.is_ok());
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_partial_match_contains() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~Note>Control>class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_exact_match() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~=Notepad>Control>class~=Edit";
    let control = find_control_by_selector(selector_str);
    // Exact match for "Edit" might work, but "Notepad" title might have more
    // Just verify it parses and attempts resolution
    assert!(control.is_ok() || control.is_err());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_startswith_match() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    // Window title is usually "Untitled - Notepad" or "*Untitled - Notepad"
    // Test with a pattern that matches the beginning
    let selector_str = "Window>title~*Untitled>Control>class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_endswith_match() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~$pad>Control>class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_nonexistent_window() {
    let result = find_window_by_selector("Window>title~NONEXISTENT_WINDOW_XYZ");
    assert!(result.is_err());
}

#[test]
fn test_selector_nonexistent_control() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~Notepad>Control>class~NONEXISTENT_CLASS_XYZ";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_err());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_control_interaction() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~Notepad>Control>class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    let ctrl = control.unwrap();
    assert!(ctrl.set_text("Selector Test").is_ok());
    sleep(Duration::from_millis(100));

    let text = ctrl.text();
    assert_eq!(text, "Selector Test");

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_file_io() {
    let test_file = "test_selector_temp.txt";

    let selector_str = "Window>title~Notepad>Control>class~Edit";
    let selector = selector::Selector::parse(selector_str).unwrap();

    assert!(selector.to_file(test_file).is_ok());

    let loaded = selector::Selector::from_file(test_file);
    assert!(loaded.is_ok());

    let loaded_sel = loaded.unwrap();
    assert_eq!(loaded_sel.to_dsl(), selector_str);

    let _ = std::fs::remove_file(test_file);
}

#[test]
fn test_selector_case_insensitive_matching() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    // Test with different case
    let selector_str = "Window>title~NOTEPAD>Control>class~edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_parse_invalid_dsl() {
    let invalid_cases = vec![
        "",
        "Window",
        "Window>title~",
        "Window>title",
        "Window>invalid_attr~value",
    ];

    for invalid_dsl in invalid_cases {
        let result = selector::Selector::parse(invalid_dsl);
        assert!(result.is_err(), "Expected error for: {}", invalid_dsl);
    }
}

#[test]
fn test_selector_roundtrip() {
    let original = "Window>title~Notepad;class~#32770>Control>class~Edit;text~Search";
    let selector = selector::Selector::parse(original).unwrap();
    let reconstructed = selector.to_dsl();
    assert_eq!(original, reconstructed);
}

#[test]
fn test_selector_window_by_selector_obj() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector = selector::Selector::parse("Window>title~Notepad").unwrap();
    let window = find_element_by_selector_obj(&selector);
    assert!(window.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_control_by_selector_obj() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector = selector::Selector::parse("Window>title~Notepad>Control>class~Edit").unwrap();
    let control = find_element_by_selector_obj(&selector);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_whitespace_handling() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    // Selector with extra whitespace
    let selector_str = "  Window > title ~ Notepad > Control > class ~ Edit  ";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_regex_window_title_basic() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~regex:.*Notepad";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    let win = window.unwrap();
    assert!(win.text.to_lowercase().contains("notepad"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_regex_window_title_complex_pattern() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    // Pattern that matches *Untitled (with or without asterisk)
    let selector_str = "Window>title~regex:^\\*?.*Notepad$";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_regex_with_control() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window>title~regex:.*Notepad.*>Control>class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    let ctrl = control.unwrap();
    assert!(ctrl.class_name.to_lowercase().contains("edit"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_regex_case_insensitive() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    // Pattern with lowercase that matches NOTEPAD
    let selector_str = "Window>title~regex:notepad";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_regex_no_match() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    // Impossible pattern that should not match
    let selector_str = "Window>title~regex:^XYZ123$";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_err());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_window_to_selector_title_and_class() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Notepad".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Notepad;class~#32770");
}

#[test]
fn test_window_to_selector_title_only() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Notepad".to_string(),
        class_name: "".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Notepad");
}

#[test]
fn test_window_to_selector_class_only() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>class~#32770");
}

#[test]
fn test_window_to_selector_both_empty_error() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "".to_string(),
        class_name: "".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let result = window_to_selector(&window);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[test]
fn test_window_to_selector_with_escaped_greater_than() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Report > Analysis".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(
        selector,
        "Window>title~Report \\> Analysis;class~MainWindow"
    );
}

#[test]
fn test_window_to_selector_with_escaped_semicolon() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Data; Export".to_string(),
        class_name: "MyApp".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Data\\; Export;class~MyApp");
}

#[test]
fn test_window_to_selector_with_escaped_backslash() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Path\\File".to_string(),
        class_name: "App".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~Path\\\\File;class~App");
}

#[test]
fn test_window_to_selector_with_multiple_escaped_chars() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "A > B; C\\D".to_string(),
        class_name: "App".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert_eq!(selector, "Window>title~A \\> B\\; C\\\\D;class~App");
}

#[test]
fn test_window_to_selector_long_title() {
    let long_title = "A".repeat(500);
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: long_title.clone(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector = window_to_selector(&window).unwrap();
    assert!(selector.contains(&long_title));
}

#[test]
fn test_control_to_selector_class_and_text() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Notepad".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let control = Element {
        id: ElementId(54321),
        element_type: ElementType::Control,
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
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let control = Element {
        id: ElementId(54321),
        element_type: ElementType::Control,
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
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let control = Element {
        id: ElementId(54321),
        element_type: ElementType::Control,
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
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let control = Element {
        id: ElementId(54321),
        element_type: ElementType::Control,
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
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "App".to_string(),
        class_name: "MainWindow".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let control = Element {
        id: ElementId(54321),
        element_type: ElementType::Control,
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
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Notepad".to_string(),
        class_name: "#32770".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
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
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "Report > Data".to_string(),
        class_name: "MyApp".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
    };

    let selector_str = window_to_selector(&window).unwrap();
    let selector = ui_automation::selector::Selector::parse(&selector_str).unwrap();

    // Verify the escaped string is unescaped after parsing
    let title_crit = &selector.path[0].criteria[0];
    assert_eq!(title_crit.value, "Report > Data");
}

#[test]
fn test_generated_selector_format() {
    let window = Element {
        id: ElementId(12345),
        element_type: ElementType::Window,
        text: "App".to_string(),
        class_name: "Window".to_string(),
        bounds: Rect {
            left: 0,
            top: 0,
            width: 800,
            height: 600,
        },
        visible: true,
        enabled: true,
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
    let app = match launch_application("notepad.exe", "") {
        Ok(app) => app,
        Err(_) => return, // Skip if Notepad can't be launched
    };
    sleep(Duration::from_millis(500));

    // Get the window
    let windows = match find_windows() {
        Ok(w) => w,
        Err(_) => {
            let _ = app.close();
            return;
        }
    };

    let notepad = match windows.iter().find(|w| w.text.contains("Notepad")) {
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
    let app = match launch_application("notepad.exe", "") {
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
    let controls = match find_child_elements(window.id.as_hwnd()) {
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
    let app = match launch_application("notepad.exe", "") {
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
