use ui_automation::{init, cleanup, Window, Element};

fn main() {
    if let Err(e) = init() {
        eprintln!("Failed to initialize UI Automation: {}", e);
        return;
    }

    println!("=== Window Operations ===");
    if let Some(window) = Window::find_by_title("Notepad") {
        println!("Found Notepad window");

        if let Ok(rect) = window.get_rect() {
            println!("Window rect: x={}, y={}, width={}, height={}",
                     rect.x, rect.y, rect.width, rect.height);
        }

        println!("Window visible: {}", window.is_visible());

        if let Err(e) = window.set_focus() {
            eprintln!("Failed to set focus: {}", e);
        }
    } else {
        println!("Notepad window not found");
    }

    println!("\n=== Element Operations ===");
    if let Some(element) = Element::find_by_name("Text Editor") {
        println!("Found Text Editor element");

        if let Ok(text) = element.get_text() {
            println!("Element text: {}", text);
        }

        println!("Element enabled: {}", element.is_enabled());

        if let Ok(rect) = element.get_rect() {
            println!("Element rect: x={}, y={}, width={}, height={}",
                     rect.x, rect.y, rect.width, rect.height);
        }
    }

    println!("\n=== All Windows ===");
    let windows = Window::get_all_windows();
    println!("Found {} visible windows", windows.len());
    for (i, window) in windows.iter().enumerate().take(5) {
        if let Ok(rect) = window.get_rect() {
            println!("Window {}: {}x{}", i, rect.width, rect.height);
        }
    }

    cleanup();
    println!("\nDone!");
}
