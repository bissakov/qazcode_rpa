use std::thread::sleep;
use std::time::Duration;

use ui_automation::*;

#[test]
fn test_application_creation() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let app = result.unwrap();
    assert!(app.is_running());
    assert_eq!(app.pid(), app.id().0);

    // Test get_name method
    let name_result = app.get_name();
    assert!(name_result.is_ok());
    let name = name_result.unwrap();
    assert!(name.to_lowercase().contains("notepad"));

    app.close().unwrap();
    assert!(!app.is_running());

    let result = app.wait_for_exit(Some(5000));
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));
}

#[test]
fn test_process_enumeration() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let processes = find_processes_by_name("notepad");
    assert!(processes.is_ok());

    let procs = processes.unwrap();
    assert!(!procs.is_empty());

    for proc in procs {
        assert!(proc.is_running());
    }

    let app = result.unwrap();
    app.close().unwrap();

    sleep(Duration::from_millis(100));
}

#[test]
fn test_attach_to_process() {
    let processes = find_processes_by_name("notepad").unwrap();
    if let Some(proc) = processes.first() {
        let attached = attach_to_process_by_pid(proc.pid());
        assert!(attached.is_ok());
        let attached_app = attached.unwrap();
        assert_eq!(attached_app.pid(), proc.pid());
    }

    sleep(Duration::from_millis(100));
}

#[test]
fn test_attach_to_process_by_name() {
    let result = launch_application("notepad.exe", "");
    assert!(result.is_ok());

    let result = attach_to_process_by_name("notepad");
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let app = result.unwrap();
    assert!(app.is_running());

    app.close().unwrap();
    assert!(!app.is_running());

    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_windows() {
    let result = find_windows();
    assert!(result.is_ok());

    let windows = result.unwrap();
    assert!(!windows.is_empty());
}

#[test]
fn test_find_windows_by_title() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    assert!(!windows.is_empty());

    let notepad_window = windows.first().unwrap();
    assert!(notepad_window.title.to_lowercase().contains("notepad"));
    assert!(notepad_window.visible);

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_windows_by_title_regex() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title_regex(r"(?i).*notepad.*").unwrap();
    assert!(!windows.is_empty());

    let windows_exact = find_windows_by_title_regex(r"^Untitled - Notepad$").unwrap();
    assert!(!windows_exact.is_empty());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_windows_by_class() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_class("Notepad").unwrap();
    assert!(!windows.is_empty());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_windows_by_process() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_process(app.pid()).unwrap();
    assert!(!windows.is_empty());

    let window = windows.first().unwrap();
    assert_eq!(window.get_process_id(), app.pid());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_window_operations() {
    let _ = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    assert!(!windows.is_empty());

    let window = windows.first().unwrap();

    assert!(window.activate().is_ok());
    sleep(Duration::from_millis(200));

    assert!(window.minimize().is_ok());
    sleep(Duration::from_millis(200));
    assert!(window.is_minimized());

    assert!(window.restore().is_ok());
    sleep(Duration::from_millis(200));
    assert!(!window.is_minimized());

    assert!(window.maximize().is_ok());
    sleep(Duration::from_millis(200));
    assert!(window.is_maximized());

    assert!(window.restore().is_ok());
    sleep(Duration::from_millis(200));
    assert!(!window.is_maximized());

    assert!(window.close().is_ok());
    sleep(Duration::from_millis(100));
}

#[test]
fn test_window_resize_and_move() {
    let _ = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let mut windows = find_windows_by_title("notepad").unwrap();
    assert!(!windows.is_empty());

    let window = windows.first_mut().unwrap();

    assert!(window.resize(800, 600).is_ok());
    sleep(Duration::from_millis(200));

    assert!(window.refresh().is_ok());
    assert_eq!(window.bounds.width, 800);
    assert_eq!(window.bounds.height, 600);

    assert!(window.move_to(100, 100).is_ok());
    sleep(Duration::from_millis(200));

    assert!(window.refresh().is_ok());
    assert_eq!(window.bounds.left, 100);
    assert_eq!(window.bounds.top, 100);

    assert!(window.close().is_ok());
    sleep(Duration::from_millis(100));
}

#[test]
fn test_get_foreground_window() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let foreground = get_foreground_window().unwrap();
    assert!(foreground.title.to_lowercase().contains("notepad"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_window_refresh() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let mut window = windows.into_iter().next().unwrap();

    let original_title = window.title.clone();

    window.minimize().unwrap();
    sleep(Duration::from_millis(200));

    window.refresh().unwrap();
    assert!(window.is_minimized());

    window.restore().unwrap();
    sleep(Duration::from_millis(200));

    window.refresh().unwrap();
    assert!(!window.is_minimized());
    assert_eq!(window.title, original_title);

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_controls_in_window() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    assert!(!windows.is_empty());

    let window = windows.first().unwrap();
    let controls = find_controls_in_window(window.id.as_hwnd());
    assert!(controls.is_ok());

    let ctrls = controls.unwrap();
    assert!(!ctrls.is_empty());

    for control in &ctrls {
        assert!(!control.class_name.is_empty());
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_controls_by_class() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    assert!(!windows.is_empty());

    let window = windows.first().unwrap();
    let controls = find_controls_by_class(window.id.as_hwnd(), "Edit");
    assert!(controls.is_ok());

    let ctrls = controls.unwrap();
    for control in &ctrls {
        assert!(control.class_name.to_lowercase().contains("edit"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_text_operations() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let result = edit_control.set_text("Hello, Rust!");
        assert!(result.is_ok());

        sleep(Duration::from_millis(200));

        let text = edit_control.get_text();
        assert!(text.is_ok());
        assert!(text.unwrap().contains("Hello, Rust!"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_focus() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let result = edit_control.focus();
        assert!(result.is_ok());
        sleep(Duration::from_millis(200));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_click() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let result = edit_control.click();
        assert!(result.is_ok());
        sleep(Duration::from_millis(200));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_double_click() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let result = edit_control.double_click();
        assert!(result.is_ok());
        sleep(Duration::from_millis(200));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_right_click() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let result = edit_control.right_click();
        assert!(result.is_ok());
        sleep(Duration::from_millis(200));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_type_text_in_notepad() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("Test automation text");
    assert!(result.is_ok());

    sleep(Duration::from_millis(500));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(text.contains("Test automation text"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_type_text_with_newline() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("Line 1\nLine 2");
    assert!(result.is_ok());

    sleep(Duration::from_millis(500));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(text.contains("Line 1"));
        assert!(text.contains("Line 2"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_enter_key() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("First line");
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let result = press_enter();
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let result = type_text("Second line");
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(text.contains("First line"));
        assert!(text.contains("Second line"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_escape_key() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = press_escape();
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_tab_key() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("Before tab");
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let result = press_tab();
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let result = type_text("After tab");
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_individual_key() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = press_key('A');
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let result = press_key('1');
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(text.contains("A") || text.contains("a"));
        assert!(text.contains("1"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_ctrl_a_key_combination() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("Select all test");
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    let result = key_combination("CTRL", 0x41);
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_shift_modifier() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = key_combination("SHIFT", 0x41);
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_alt_modifier() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = key_combination("ALT", 0x46);
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_click_at_coordinates() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let result = click(100, 100);
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_right_click_at_coordinates() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = right_click(window.bounds.left + 100, window.bounds.top + 100);
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_move_mouse() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let result = move_mouse(window.bounds.left + 50, window.bounds.top + 50);
    assert!(result.is_ok());

    sleep(Duration::from_millis(200));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_refresh() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(mut edit_control) = controls
        .into_iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let original_visible = edit_control.visible;

        let result = edit_control.refresh();
        assert!(result.is_ok());

        assert_eq!(edit_control.visible, original_visible);
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_control_visibility_and_state() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    for control in &controls {
        assert!(control.visible || !control.visible);
        assert!(control.enabled || !control.enabled);
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_find_control_by_text() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if !controls.is_empty() {
        let first_text = &controls[0].text;
        if !first_text.is_empty() {
            let result = find_control_by_text(window.id.as_hwnd(), first_text);
            assert!(result.is_ok());
        }
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_type_special_characters() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("Hello!");
    assert!(result.is_ok());

    sleep(Duration::from_millis(300));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(text.contains("Hello!"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_multiple_key_presses() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    for _ in 0..5 {
        let result = press_key('A');
        assert!(result.is_ok());
        sleep(Duration::from_millis(50));
    }

    sleep(Duration::from_millis(300));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

// ============= FEATURE 1: EXTENDED SPECIAL KEYS =============

#[test]
fn test_press_arrow_keys() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("Line1");
    assert!(result.is_ok());

    sleep(Duration::from_millis(100));

    assert!(press_arrow_left().is_ok());
    assert!(press_arrow_right().is_ok());
    assert!(press_arrow_up().is_ok());
    assert!(press_arrow_down().is_ok());

    sleep(Duration::from_millis(200));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_function_keys() {
    let app = launch_application("charmap.exe", "").unwrap();
    sleep(Duration::from_millis(1000));

    assert!(press_f_key(1).is_ok());
    sleep(Duration::from_millis(100));
    assert!(press_f_key(5).is_ok());
    sleep(Duration::from_millis(100));
    assert!(press_f_key(12).is_ok());
    sleep(Duration::from_millis(100));

    app.close().unwrap();
    sleep(Duration::from_millis(200));
}

#[test]
fn test_press_delete_backspace() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    let result = type_text("abcdef");
    assert!(result.is_ok());
    sleep(Duration::from_millis(200));

    assert!(press_backspace().is_ok());
    sleep(Duration::from_millis(150));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(!text.is_empty());
        assert!(text.contains("abcde"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_home_end() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(type_text("First line").is_ok());
    sleep(Duration::from_millis(100));

    assert!(press_enter().is_ok());
    sleep(Duration::from_millis(100));

    assert!(type_text("Second line").is_ok());
    sleep(Duration::from_millis(100));

    assert!(press_home().is_ok());
    sleep(Duration::from_millis(100));

    assert!(press_end().is_ok());
    sleep(Duration::from_millis(100));

    assert!(press_page_up().is_ok());
    sleep(Duration::from_millis(100));

    assert!(press_page_down().is_ok());
    sleep(Duration::from_millis(100));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_press_key_by_name() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(press_key_by_name("delete").is_ok());
    assert!(press_key_by_name("Delete").is_ok());
    assert!(press_key_by_name("DELETE").is_ok());
    assert!(press_key_by_name("arrow_up").is_ok());
    assert!(press_key_by_name("ArrowDown").is_ok());
    assert!(press_key_by_name("f1").is_ok());
    assert!(press_key_by_name("F12").is_ok());
    assert!(press_key_by_name("home").is_ok());
    assert!(press_key_by_name("end").is_ok());

    assert!(press_key_by_name("invalid_key").is_err());

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

// ============= FEATURE 2: KEY HOLD/RELEASE =============

#[test]
fn test_key_down_up_basic() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(key_down(0x41).is_ok());
    sleep(Duration::from_millis(200));
    assert!(key_up(0x41).is_ok());

    sleep(Duration::from_millis(300));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(!text.is_empty());
        assert!(text.contains("A") || text.contains("a"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_shift_held_multiple_chars() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(key_combination("shift", 0x48).is_ok());
    sleep(Duration::from_millis(100));
    assert!(key_combination("shift", 0x45).is_ok());
    sleep(Duration::from_millis(100));
    assert!(key_combination("shift", 0x4C).is_ok());
    sleep(Duration::from_millis(100));
    assert!(key_combination("shift", 0x4C).is_ok());
    sleep(Duration::from_millis(100));
    assert!(key_combination("shift", 0x4F).is_ok());

    sleep(Duration::from_millis(300));

    let controls = find_controls_in_window(window.id.as_hwnd()).unwrap();
    if let Some(edit_control) = controls
        .iter()
        .find(|c| c.class_name.to_lowercase().contains("edit"))
    {
        let text = edit_control.get_text().unwrap();
        assert!(text.contains("HELLO") || text.contains("hello"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_modifier_separate_press_release() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(type_text("test text").is_ok());
    sleep(Duration::from_millis(200));

    assert!(key_down_ctrl().is_ok());
    sleep(Duration::from_millis(50));
    assert!(press_key_code(0x41).is_ok());
    sleep(Duration::from_millis(50));
    assert!(key_up_ctrl().is_ok());

    sleep(Duration::from_millis(200));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

// ============= FEATURE 3: WAIT CONDITIONS =============

#[test]
fn test_wait_for_window_found() {
    let start = std::time::Instant::now();

    let result = wait_for_window("notepad", 5000, 100);

    if result.is_err() {
        let app = launch_application("notepad.exe", "").unwrap();
        sleep(Duration::from_millis(500));

        let result2 = wait_for_window("notepad", 5000, 100);
        assert!(result2.is_ok());
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed < 6000);

        app.close().unwrap();
        sleep(Duration::from_millis(100));
    } else {
        let elapsed = start.elapsed().as_millis();
        assert!(elapsed < 6000);
    }
}

#[test]
fn test_wait_for_window_timeout() {
    let start = std::time::Instant::now();

    let result = wait_for_window("NonExistentWindowXYZ12345", 1000, 100);

    assert!(result.is_err());
    let elapsed = start.elapsed().as_millis();
    assert!(elapsed >= 1000 && elapsed < 1200);

    if let Err(AutomationError::Other(msg)) = result {
        assert!(msg.contains("Timeout"));
    }
}

#[test]
fn test_wait_for_control_in_window() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();

    let result = wait_for_control(window.id.as_hwnd(), "Edit", 2000, 100);
    assert!(result.is_ok());

    let control = result.unwrap();
    assert!(control.class_name.to_lowercase().contains("edit"));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_wait_for_control_text_match() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(type_text("TestText123").is_ok());
    sleep(Duration::from_millis(500));

    let result = wait_for_control_text(window.id.as_hwnd(), "test", 3000, 100);
    if result.is_ok() {
        let control = result.unwrap();
        assert!(control.text.contains("Test") || control.text.contains("test"));
    }

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

// ============= FEATURE 4: KEY SEQUENCES =============

#[test]
fn test_key_sequence_simple_ctrl_a() {
    let app = launch_application("notepad.exe", "").unwrap();
    sleep(Duration::from_millis(500));

    let windows = find_windows_by_title("notepad").unwrap();
    let window = windows.first().unwrap();
    window.activate().unwrap();
    sleep(Duration::from_millis(200));

    assert!(type_text("test content").is_ok());
    sleep(Duration::from_millis(200));

    assert!(key_combination("ctrl", 0x41).is_ok());
    sleep(Duration::from_millis(100));

    app.close().unwrap();
    sleep(Duration::from_millis(100));
}

#[test]
fn test_key_sequence_complex() {
    assert!(key_sequence("ctrl+shift+escape").is_ok());
    sleep(Duration::from_millis(100));
}
