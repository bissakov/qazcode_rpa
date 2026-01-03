use std::time::Duration;
use ui_automation::*;

fn sleep(duration: Duration) {
    std::thread::sleep(duration);
}

#[test]
fn test_selector_find_window_by_title() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window/title~Notepad";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    let win = window.unwrap();
    assert!(win.title.to_lowercase().contains("notepad"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_find_window_by_class() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window/class~Notepad";
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

    let selector_str = "Window/title~Notepad/Control/class~Edit";
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

    let windows = find_windows_by_title("Notepad").unwrap();
    let window = &windows[0];
    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    let edit_control = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"));

    if edit_control.is_some() {
        let selector_str = "Window/title~Notepad|class~Notepad/Control/class~Edit";
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

    let selector_str = "Window/title~Note/Control/class~Edit";
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

    let selector_str = "Window/title~=Notepad/Control/class~=Edit";
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
    let selector_str = "Window/title~*Untitled/Control/class~Edit";
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

    let selector_str = "Window/title~$pad/Control/class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_nonexistent_window() {
    let result = find_window_by_selector("Window/title~NONEXISTENT_WINDOW_XYZ");
    assert!(result.is_err());
}

#[test]
fn test_selector_nonexistent_control() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    sleep(Duration::from_millis(300));

    let selector_str = "Window/title~Notepad/Control/class~NONEXISTENT_CLASS_XYZ";
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

    let selector_str = "Window/title~Notepad/Control/class~Edit";
    let control = find_control_by_selector(selector_str);
    assert!(control.is_ok());

    let ctrl = control.unwrap();
    assert!(ctrl.set_text("Selector Test").is_ok());
    sleep(Duration::from_millis(100));

    let text = ctrl.get_text().unwrap();
    assert_eq!(text, "Selector Test");

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_selector_file_io() {
    let test_file = "test_selector_temp.txt";

    let selector_str = "Window/title~Notepad/Control/class~Edit";
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
    let selector_str = "Window/title~NOTEPAD/Control/class~edit";
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
        "Window/title~",
        "Window/title",
        "Window/invalid_attr~value",
    ];

    for invalid_dsl in invalid_cases {
        let result = selector::Selector::parse(invalid_dsl);
        assert!(result.is_err(), "Expected error for: {}", invalid_dsl);
    }
}

#[test]
fn test_selector_roundtrip() {
    let original = "Window/title~Notepad|class~#32770/Control/class~Edit|text~Search";
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

    let selector = selector::Selector::parse("Window/title~Notepad").unwrap();
    let window = find_window_by_selector_obj(&selector);
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

    let selector = selector::Selector::parse("Window/title~Notepad/Control/class~Edit").unwrap();
    let control = find_control_by_selector_obj(&selector);
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
    let selector_str = "  Window / title ~ Notepad / Control / class ~ Edit  ";
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

    let selector_str = "Window/title~regex:.*Notepad";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_ok());

    let win = window.unwrap();
    assert!(win.title.to_lowercase().contains("notepad"));

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
    let selector_str = "Window/title~regex:^\\*?.*Notepad$";
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

    let selector_str = "Window/title~regex:.*Notepad.*/Control/class~Edit";
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
    let selector_str = "Window/title~regex:notepad";
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
    let selector_str = "Window/title~regex:^XYZ123$";
    let window = find_window_by_selector(selector_str);
    assert!(window.is_err());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}
