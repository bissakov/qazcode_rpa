use crate::{AutomationError, Control, Window};

/// Generate a selector DSL string from a Window
///
/// Generates a selector with both title and class criteria for robustness.
/// Special characters (>, ;, \) in values are automatically escaped.
///
/// Returns error if both title and class are empty.
pub fn window_to_selector(window: &Window) -> Result<String, AutomationError> {
    let title_empty = window.title.is_empty();
    let class_empty = window.class_name.is_empty();

    if title_empty && class_empty {
        return Err(AutomationError::Other(
            "Cannot generate window selector: both title and class are empty".to_string(),
        ));
    }

    let mut criteria = Vec::new();

    if !title_empty {
        let escaped_title = super::selector::escape_dsl_value(&window.title);
        criteria.push(format!("title~{}", escaped_title));
    }

    if !class_empty {
        let escaped_class = super::selector::escape_dsl_value(&window.class_name);
        criteria.push(format!("class~{}", escaped_class));
    }

    let criteria_str = criteria.join(";");
    Ok(format!("Window>{}", criteria_str))
}

/// Generate a selector DSL string from a Control and its parent Window
///
/// Generates a selector with class, text, and index criteria.
/// Index is included if the control is not the first with its class.
/// Special characters (>, ;, \) in values are automatically escaped.
///
/// Returns error if both class and text are empty.
pub fn control_to_selector(
    control: &Control,
    parent_window: &Window,
) -> Result<String, AutomationError> {
    let text_empty = control.text.is_empty();
    let class_empty = control.class_name.is_empty();

    if text_empty && class_empty {
        return Err(AutomationError::Other(
            "Cannot generate control selector: both class and text are empty".to_string(),
        ));
    }

    // Generate parent window selector
    let window_dsl = window_to_selector(parent_window)?;

    let mut criteria = Vec::new();

    if !class_empty {
        let escaped_class = super::selector::escape_dsl_value(&control.class_name);
        criteria.push(format!("class~{}", escaped_class));
    }

    if !text_empty {
        let escaped_text = super::selector::escape_dsl_value(&control.text);
        criteria.push(format!("text~{}", escaped_text));
    }

    // Calculate control index among siblings with same class
    match get_control_index(control, parent_window) {
        Ok(index) => {
            // Include index if multiple controls with same class exist (i.e., index > 0)
            if index > 0 {
                criteria.push(format!("index~{}", index));
            }
        }
        Err(_) => {
            // If we can't calculate index, continue without it
            // This can happen if the control is not found, but we still generate the selector
        }
    }

    let criteria_str = criteria.join(";");
    Ok(format!("{}>Control>{}", window_dsl, criteria_str))
}

/// Calculate control index among siblings with the same class
fn get_control_index(control: &Control, parent_window: &Window) -> Result<usize, AutomationError> {
    // Find all controls in parent window
    let siblings = crate::find_controls_in_window(parent_window.id.as_hwnd())?;

    // Filter to controls with same class (case-insensitive)
    let control_class_lower = control.class_name.to_lowercase();
    let same_class: Vec<_> = siblings
        .iter()
        .filter(|c| c.class_name.to_lowercase() == control_class_lower)
        .collect();

    // Find target control's position in filtered list
    for (idx, sibling) in same_class.iter().enumerate() {
        if sibling.id.0 == control.id.0 {
            return Ok(idx);
        }
    }

    Err(AutomationError::Other(
        "Control not found in parent window".to_string(),
    ))
}
