use crate::automation::{AutomationError, Control, Window, find_controls_in_window};
use regex::Regex;
use std::fs;
use std::path::Path;

/// How to match attribute values
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Exact,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
}

impl MatchType {
    /// Parse match type from operator: ~, ~=, ~*, ~$
    fn from_operator(op: &str) -> Self {
        match op {
            "~=" => MatchType::Exact,
            "~*" => MatchType::StartsWith,
            "~$" => MatchType::EndsWith,
            _ => MatchType::Contains, // Default
        }
    }
}

/// Single matching criterion (attribute + value + match type)
#[derive(Debug, Clone)]
pub struct SelectorCriteria {
    pub attribute: String,
    pub value: String,
    pub match_type: MatchType,
    pub regex_obj: Option<Regex>,
}

/// One level in selector path (Window or Control)
#[derive(Debug, Clone)]
pub struct SelectorPath {
    pub element_type: String, // "Window" or "Control"
    pub criteria: Vec<SelectorCriteria>,
}

/// Parsed selector: sequence of paths from Window → Control → ...
#[derive(Debug, Clone)]
pub struct Selector {
    pub path: Vec<SelectorPath>,
    pub original: String,
}

impl Selector {
    /// Parse DSL string into Selector
    /// Format: "Window>attr~val;attr~val>Control>attr~val;attr~val"
    /// Special characters in values are escaped: \>, \;, \\
    pub fn parse(dsl: &str) -> Result<Self, AutomationError> {
        let trimmed = dsl.trim();
        if trimmed.is_empty() {
            return Err(AutomationError::Other(
                "Selector DSL cannot be empty".to_string(),
            ));
        }

        let mut path = Vec::new();
        let mut current_element_type: Option<String> = None;
        let mut current_criteria = Vec::new();

        // Split by unescaped '>' to get element types and criteria
        let parts = split_by_unescaped(trimmed, '>');

        if parts.is_empty() {
            return Err(AutomationError::Other(
                "Invalid selector DSL format".to_string(),
            ));
        }

        for part in parts {
            let part = part.trim();

            // Check if this is an element type
            if part == "Window" || part == "Control" {
                // Save previous element if exists
                if let Some(element_type) = current_element_type.take() {
                    if current_criteria.is_empty() {
                        return Err(AutomationError::Other(format!(
                            "Element '{}' has no criteria",
                            element_type
                        )));
                    }
                    path.push(SelectorPath {
                        element_type,
                        criteria: current_criteria.clone(),
                    });
                    current_criteria.clear();
                }

                current_element_type = Some(part.to_string());
            } else if !part.is_empty() {
                // Split by unescaped ';' for multiple criteria
                let criteria_parts = split_by_unescaped(part, ';');
                for criteria_part in criteria_parts {
                    let criteria = parse_criteria(&criteria_part)?;
                    current_criteria.push(criteria);
                }
            }
        }

        // Save final element
        if let Some(element_type) = current_element_type {
            if current_criteria.is_empty() {
                return Err(AutomationError::Other(format!(
                    "Element '{}' has no criteria",
                    element_type
                )));
            }
            path.push(SelectorPath {
                element_type,
                criteria: current_criteria,
            });
        } else {
            return Err(AutomationError::Other(
                "No valid element types found in selector".to_string(),
            ));
        }

        if path.is_empty() {
            return Err(AutomationError::Other(
                "Selector must have at least one element (Window or Control)".to_string(),
            ));
        }

        Ok(Selector {
            path,
            original: trimmed.to_string(),
        })
    }

    /// Convert selector back to DSL string
    pub fn to_dsl(&self) -> String {
        self.original.clone()
    }

    /// Load selector from file (reads first line as DSL)
    pub fn from_file(path: &str) -> Result<Self, AutomationError> {
        let content = fs::read_to_string(path)
            .map_err(|e| AutomationError::Other(format!("Failed to read selector file: {}", e)))?;

        let dsl = content.lines().next().unwrap_or("").trim();
        if dsl.is_empty() {
            return Err(AutomationError::Other(
                "Selector file is empty or contains no DSL".to_string(),
            ));
        }

        Selector::parse(dsl)
    }

    /// Save selector to file
    pub fn to_file(&self, path: &str) -> Result<(), AutomationError> {
        // Create parent directories if needed
        if let Some(parent) = Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).map_err(|e| {
                    AutomationError::Other(format!("Failed to create directory: {}", e))
                })?;
            }
        }

        fs::write(path, &self.original)
            .map_err(|e| AutomationError::Other(format!("Failed to write selector file: {}", e)))?;

        Ok(())
    }
}

/// Split string by delimiter character, respecting escape sequences (backslash)
fn split_by_unescaped(s: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < s.len() {
        if bytes[i] == b'\\' && i + 1 < s.len() {
            i += 2;
        } else if s.chars().nth(i) == Some(delimiter) {
            parts.push(&s[start..i]);
            start = i + 1;
            i += 1;
        } else {
            i += 1;
        }
    }

    parts.push(&s[start..]);
    parts
}

/// Escape special DSL characters in values: >, ;, \
/// Used when generating selectors from element properties
pub fn escape_dsl_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('>', "\\>")
        .replace(';', "\\;")
}

/// Unescape special DSL characters in values
/// Used when parsing selectors to recover original values
fn unescape_dsl_value(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(&next) = chars.peek() {
                match next {
                    '>' | ';' | '\\' => {
                        chars.next();
                        result.push(next);
                    }
                    _ => result.push(ch),
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Parse single criteria string: "attr~val" or "attr~=val" or "attr~*val" or "attr~$val" or "attr~regex:pattern"
fn parse_criteria(criteria_str: &str) -> Result<SelectorCriteria, AutomationError> {
    let trimmed = criteria_str.trim();

    // Try to find operator: ~=, ~*, ~$, or ~
    let (attribute, operator, value) = if let Some(pos) = trimmed.find("~=") {
        let attr = trimmed[..pos].trim();
        let val = trimmed[pos + 2..].trim();
        (attr, "~=", val)
    } else if let Some(pos) = trimmed.find("~*") {
        let attr = trimmed[..pos].trim();
        let val = trimmed[pos + 2..].trim();
        (attr, "~*", val)
    } else if let Some(pos) = trimmed.find("~$") {
        let attr = trimmed[..pos].trim();
        let val = trimmed[pos + 2..].trim();
        (attr, "~$", val)
    } else if let Some(pos) = trimmed.find('~') {
        let attr = trimmed[..pos].trim();
        let val = trimmed[pos + 1..].trim();
        (attr, "~", val)
    } else {
        return Err(AutomationError::Other(format!(
            "Invalid criteria format (missing ~): {}",
            trimmed
        )));
    };

    if attribute.is_empty() || value.is_empty() {
        return Err(AutomationError::Other(format!(
            "Criteria has empty attribute or value: {}",
            trimmed
        )));
    }

    // Validate attribute names
    let attr_lower = attribute.to_lowercase();
    if !matches!(attr_lower.as_str(), "title" | "class" | "text" | "index") {
        return Err(AutomationError::Other(format!(
            "Unknown attribute: {}. Valid: title, class, text, index",
            attribute
        )));
    }

    // Check if value uses regex syntax (only allowed for title)
    let (match_type, regex_obj) = if value.starts_with("regex:") {
        // Regex pattern support - only for title attribute
        if attr_lower != "title" {
            return Err(AutomationError::Other(
                "Regex patterns are only supported for 'title' attribute".to_string(),
            ));
        }

        let pattern = &value[6..]; // Remove "regex:" prefix
        if pattern.is_empty() {
            return Err(AutomationError::Other(
                "Regex pattern cannot be empty".to_string(),
            ));
        }

        // Compile regex with case-insensitive flag
        let regex = match Regex::new(&format!("(?i){}", pattern)) {
            Ok(r) => r,
            Err(e) => {
                return Err(AutomationError::Other(format!(
                    "Invalid regex pattern: {}",
                    e
                )));
            }
        };

        (MatchType::Regex, Some(regex))
    } else {
        let match_type = MatchType::from_operator(operator);
        (match_type, None)
    };

    // Unescape the value
    let unescaped_value = unescape_dsl_value(value);

    Ok(SelectorCriteria {
        attribute: attr_lower,
        value: unescaped_value,
        match_type,
        regex_obj,
    })
}

/// Evaluate if string matches criteria
pub fn match_string(
    haystack: &str,
    needle: &str,
    match_type: MatchType,
    regex_obj: Option<&Regex>,
) -> bool {
    match match_type {
        MatchType::Regex => {
            if let Some(regex) = regex_obj {
                regex.is_match(haystack)
            } else {
                false
            }
        }
        _ => {
            let h = haystack.to_lowercase();
            let n = needle.to_lowercase();

            match match_type {
                MatchType::Exact => h == n,
                MatchType::Contains => h.contains(&n),
                MatchType::StartsWith => h.starts_with(&n),
                MatchType::EndsWith => h.ends_with(&n),
                MatchType::Regex => unreachable!(),
            }
        }
    }
}

/// Check if a window matches the given criteria
pub fn window_matches_criteria(
    title: &str,
    class_name: &str,
    criteria: &[SelectorCriteria],
) -> bool {
    criteria.iter().all(|c| match c.attribute.as_str() {
        "title" => match_string(title, &c.value, c.match_type, c.regex_obj.as_ref()),
        "class" => match_string(class_name, &c.value, c.match_type, None),
        _ => false,
    })
}

/// Check if a control matches the given criteria
pub fn control_matches_criteria(
    text: &str,
    class_name: &str,
    criteria: &[SelectorCriteria],
) -> bool {
    criteria.iter().all(|c| {
        match c.attribute.as_str() {
            "text" => match_string(text, &c.value, c.match_type, None),
            "class" => match_string(class_name, &c.value, c.match_type, None),
            "index" => {
                // Index matching would need the actual index, handled separately
                false
            }
            _ => false,
        }
    })
}

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
    let siblings = find_controls_in_window(parent_window.id.as_hwnd())?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_window() {
        let selector = Selector::parse("Window>title~Notepad").unwrap();
        assert_eq!(selector.path.len(), 1);
        assert_eq!(selector.path[0].element_type, "Window");
        assert_eq!(selector.path[0].criteria.len(), 1);
        assert_eq!(selector.path[0].criteria[0].attribute, "title");
        assert_eq!(selector.path[0].criteria[0].value, "Notepad");
    }

    #[test]
    fn test_parse_window_and_control() {
        let selector = Selector::parse("Window>title~Notepad>Control>class~Edit").unwrap();
        assert_eq!(selector.path.len(), 2);
        assert_eq!(selector.path[0].element_type, "Window");
        assert_eq!(selector.path[1].element_type, "Control");
    }

    #[test]
    fn test_parse_multiple_criteria() {
        let selector =
            Selector::parse("Window>title~Notepad;class~#32770>Control>class~Edit").unwrap();
        assert_eq!(selector.path[0].criteria.len(), 2);
        assert_eq!(selector.path[0].criteria[0].attribute, "title");
        assert_eq!(selector.path[0].criteria[1].attribute, "class");
    }

    #[test]
    fn test_parse_with_operators() {
        let selector = Selector::parse("Window>title~=Notepad>Control>text~*Save").unwrap();
        assert_eq!(selector.path[0].criteria[0].match_type, MatchType::Exact);
        assert_eq!(
            selector.path[1].criteria[0].match_type,
            MatchType::StartsWith
        );
    }

    #[test]
    fn test_parse_invalid_attribute() {
        let result = Selector::parse("Window>invalid~value");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_selector() {
        let result = Selector::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_no_criteria() {
        let result = Selector::parse("Window");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_operator() {
        let result = Selector::parse("Window>title-Notepad");
        assert!(result.is_err());
    }

    #[test]
    fn test_match_string_exact() {
        assert!(match_string("Notepad", "Notepad", MatchType::Exact, None));
        assert!(!match_string("Notepad", "Note", MatchType::Exact, None));
        assert!(match_string("notepad", "NOTEPAD", MatchType::Exact, None)); // case-insensitive
    }

    #[test]
    fn test_match_string_contains() {
        assert!(match_string("Notepad", "Note", MatchType::Contains, None));
        assert!(match_string("Notepad", "pad", MatchType::Contains, None));
        assert!(!match_string("Notepad", "xyz", MatchType::Contains, None));
    }

    #[test]
    fn test_match_string_startswith() {
        assert!(match_string("Notepad", "Note", MatchType::StartsWith, None));
        assert!(!match_string("Notepad", "pad", MatchType::StartsWith, None));
    }

    #[test]
    fn test_match_string_endswith() {
        assert!(match_string("Notepad", "pad", MatchType::EndsWith, None));
        assert!(!match_string("Notepad", "Note", MatchType::EndsWith, None));
    }

    #[test]
    fn test_to_dsl() {
        let original = "Window>title~Notepad>Control>class~Edit";
        let selector = Selector::parse(original).unwrap();
        assert_eq!(selector.to_dsl(), original);
    }

    #[test]
    fn test_parse_nested_controls() {
        let selector = Selector::parse(
            "Window>title~MyApp>Control>class~GroupBox>Control>class~Button;text~OK",
        )
        .unwrap();
        assert_eq!(selector.path.len(), 3);
        assert_eq!(selector.path[2].criteria.len(), 2);
    }

    #[test]
    fn test_parse_all_match_types() {
        let cases = vec![
            ("Window>title~Contains", MatchType::Contains),
            ("Window>title~=Exact", MatchType::Exact),
            ("Window>title~*StartsWith", MatchType::StartsWith),
            ("Window>title~$EndsWith", MatchType::EndsWith),
        ];

        for (dsl, expected_type) in cases {
            let selector = Selector::parse(dsl).unwrap();
            assert_eq!(selector.path[0].criteria[0].match_type, expected_type);
        }
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let selector =
            Selector::parse("  Window > title ~ Notepad > Control > class ~ Edit  ").unwrap();
        assert_eq!(selector.path.len(), 2);
        assert_eq!(selector.path[0].criteria[0].value, "Notepad");
    }

    #[test]
    fn test_criteria_parsing_errors() {
        let invalid_cases = vec![
            "Window>title",       // Missing value
            "Window>title~",      // Empty value
            "Window>~value",      // Missing attribute
            "Window>title-value", // Wrong operator
        ];

        for invalid_dsl in invalid_cases {
            let result = Selector::parse(invalid_dsl);
            assert!(result.is_err(), "Expected error for: {}", invalid_dsl);
        }
    }

    #[test]
    fn test_parse_regex_pattern_simple() {
        let selector = Selector::parse("Window>title~regex:.*Notepad.*").unwrap();
        assert_eq!(selector.path[0].criteria[0].match_type, MatchType::Regex);
        assert!(selector.path[0].criteria[0].regex_obj.is_some());
    }

    #[test]
    fn test_parse_regex_pattern_with_capture_groups() {
        let selector = Selector::parse("Window>title~regex:^(.*Notepad.*)$").unwrap();
        assert_eq!(selector.path[0].criteria[0].match_type, MatchType::Regex);
        assert!(selector.path[0].criteria[0].regex_obj.is_some());
        assert_eq!(selector.path[0].criteria.len(), 1);
    }

    #[test]
    fn test_parse_regex_invalid_syntax() {
        let result = Selector::parse("Window>title~regex:[invalid");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Invalid regex"));
    }

    #[test]
    fn test_parse_regex_only_on_title() {
        let result = Selector::parse("Window>class~regex:.*");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("only supported for 'title'"));
    }

    #[test]
    fn test_parse_regex_empty_pattern() {
        let result = Selector::parse("Window>title~regex:");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("empty"));
    }

    #[test]
    fn test_parse_mixed_regex_and_literal() {
        let selector = Selector::parse("Window>title~regex:.*;class~Notepad").unwrap();
        assert_eq!(selector.path[0].criteria.len(), 2);
        assert_eq!(selector.path[0].criteria[0].match_type, MatchType::Regex);
        assert_eq!(selector.path[0].criteria[1].match_type, MatchType::Contains);
    }

    #[test]
    fn test_match_string_regex_basic() {
        let regex = Regex::new("(?i).*Notepad").unwrap();
        assert!(match_string(
            "Untitled - Notepad",
            "",
            MatchType::Regex,
            Some(&regex)
        ));
    }

    #[test]
    fn test_match_string_regex_case_insensitive() {
        let regex = Regex::new("(?i)notepad").unwrap();
        assert!(match_string(
            "UNTITLED - NOTEPAD",
            "",
            MatchType::Regex,
            Some(&regex)
        ));
        assert!(match_string("notepad", "", MatchType::Regex, Some(&regex)));
    }

    #[test]
    fn test_match_string_regex_no_match() {
        let regex = Regex::new("(?i)^Untitled").unwrap();
        assert!(!match_string("Notepad", "", MatchType::Regex, Some(&regex)));
    }

    #[test]
    fn test_match_string_regex_special_chars() {
        let regex = Regex::new("(?i)\\[.*\\]").unwrap();
        assert!(match_string(
            "File [1].txt",
            "",
            MatchType::Regex,
            Some(&regex)
        ));
    }

    #[test]
    fn test_window_matches_criteria_regex() {
        let regex = Regex::new("(?i).*notepad").unwrap();
        let criteria = vec![SelectorCriteria {
            attribute: "title".to_string(),
            value: "regex:.*notepad".to_string(),
            match_type: MatchType::Regex,
            regex_obj: Some(regex),
        }];

        assert!(window_matches_criteria(
            "Untitled - Notepad",
            "#32770",
            &criteria
        ));
        assert!(!window_matches_criteria("WordPad", "#32770", &criteria));
    }

    #[test]
    fn test_to_dsl_preserves_regex() {
        let original = "Window>title~regex:.*Notepad.*";
        let selector = Selector::parse(original).unwrap();
        assert_eq!(selector.to_dsl(), original);
    }

    #[test]
    fn test_escape_dsl_value_greater_than() {
        assert_eq!(escape_dsl_value("Report > Analysis"), "Report \\> Analysis");
    }

    #[test]
    fn test_escape_dsl_value_semicolon() {
        assert_eq!(escape_dsl_value("Data; Export"), "Data\\; Export");
    }

    #[test]
    fn test_escape_dsl_value_backslash() {
        assert_eq!(escape_dsl_value("Path\\File"), "Path\\\\File");
    }

    #[test]
    fn test_escape_dsl_value_multiple() {
        assert_eq!(escape_dsl_value("A > B; C\\D"), "A \\> B\\; C\\\\D");
    }

    #[test]
    fn test_escape_dsl_value_no_special() {
        assert_eq!(escape_dsl_value("Normal text"), "Normal text");
    }

    #[test]
    fn test_unescape_dsl_value() {
        assert_eq!(
            unescape_dsl_value("Report \\> Analysis"),
            "Report > Analysis"
        );
        assert_eq!(unescape_dsl_value("Data\\; Export"), "Data; Export");
        assert_eq!(unescape_dsl_value("Path\\\\File"), "Path\\File");
    }

    #[test]
    fn test_parse_with_escaped_greater_than() {
        let selector = Selector::parse("Window>title~Report \\> Analysis;class~MyApp").unwrap();
        assert_eq!(selector.path[0].criteria[0].value, "Report > Analysis");
        assert_eq!(selector.path[0].criteria[1].value, "MyApp");
    }

    #[test]
    fn test_parse_with_escaped_semicolon() {
        let selector = Selector::parse("Window>title~Data\\; Export").unwrap();
        assert_eq!(selector.path[0].criteria[0].value, "Data; Export");
    }

    #[test]
    fn test_parse_roundtrip_with_escaped_chars() {
        let escaped_title = "Report \\> Analysis";
        let selector = Selector::parse(&format!("Window>title~{}", escaped_title)).unwrap();
        assert_eq!(selector.path[0].criteria[0].value, "Report > Analysis");
    }
}
