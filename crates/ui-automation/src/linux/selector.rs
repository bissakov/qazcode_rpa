use super::automation::{AutomationError, Element, ElementType};
use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Exact,
    Contains,
    StartsWith,
    EndsWith,
    Regex,
}

impl MatchType {
    fn from_operator(op: &str) -> Self {
        match op {
            "~=" => Self::Exact,
            "~*" => Self::StartsWith,
            "~$" => Self::EndsWith,
            _ => Self::Contains,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SelectorCriteria {
    pub attribute: String,
    pub value: String,
    pub match_type: MatchType,
    pub regex_obj: Option<Regex>,
}

#[derive(Debug, Clone)]
pub struct SelectorPath {
    pub element_type: String,
    pub criteria: Vec<SelectorCriteria>,
}

#[derive(Debug, Clone)]
pub struct Selector {
    pub path: Vec<SelectorPath>,
    pub original: String,
}

impl Selector {
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

        let parts = split_by_unescaped(trimmed, '>');

        if parts.is_empty() {
            return Err(AutomationError::Other(
                "Invalid selector DSL format".to_string(),
            ));
        }

        for part in parts {
            let part = part.trim();

            if part == "Window" || part == "Control" {
                if let Some(element_type) = current_element_type.take() {
                    if current_criteria.is_empty() {
                        return Err(AutomationError::Other(format!(
                            "Element '{element_type}' has no criteria"
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
                let criteria_parts = split_by_unescaped(part, ';');
                for criteria_part in criteria_parts {
                    let criteria = parse_criteria(criteria_part)?;
                    current_criteria.push(criteria);
                }
            }
        }

        if let Some(element_type) = current_element_type {
            if current_criteria.is_empty() {
                return Err(AutomationError::Other(format!(
                    "Element '{element_type}' has no criteria",
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

        Ok(Self {
            path,
            original: trimmed.to_string(),
        })
    }

    #[must_use]
    pub fn to_dsl(&self) -> String {
        self.original.clone()
    }

    pub fn from_file(path: &str) -> Result<Self, AutomationError> {
        let content = fs::read_to_string(path)
            .map_err(|e| AutomationError::Other(format!("Failed to read selector file: {e}")))?;

        let dsl = content.lines().next().unwrap_or("").trim();
        if dsl.is_empty() {
            return Err(AutomationError::Other(
                "Selector file is empty or contains no DSL".to_string(),
            ));
        }

        Self::parse(dsl)
    }

    pub fn to_file(&self, path: &str) -> Result<(), AutomationError> {
        if let Some(parent) = Path::new(path).parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)
                .map_err(|e| AutomationError::Other(format!("Failed to create directory: {e}")))?;
        }

        fs::write(path, &self.original)
            .map_err(|e| AutomationError::Other(format!("Failed to write selector file: {e}")))?;

        Ok(())
    }
}

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

#[must_use]
pub fn escape_dsl_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('>', "\\>")
        .replace(';', "\\;")
}

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

fn parse_criteria(criteria_str: &str) -> Result<SelectorCriteria, AutomationError> {
    let trimmed = criteria_str.trim();

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
            "Invalid criteria format (missing ~): {trimmed}"
        )));
    };

    if attribute.is_empty() || value.is_empty() {
        return Err(AutomationError::Other(format!(
            "Criteria has empty attribute or value: {trimmed}"
        )));
    }

    let attr_lower = attribute.to_lowercase();
    if !matches!(attr_lower.as_str(), "title" | "class" | "text" | "index") {
        return Err(AutomationError::Other(format!(
            "Unknown attribute: {attribute}. Valid: title, class, text, index"
        )));
    }

    let (match_type, regex_obj) = if let Some(pattern) = value.strip_prefix("regex:") {
        if attr_lower != "title" {
            return Err(AutomationError::Other(
                "Regex patterns are only supported for 'title' attribute".to_string(),
            ));
        }

        if pattern.is_empty() {
            return Err(AutomationError::Other(
                "Regex pattern cannot be empty".to_string(),
            ));
        }

        let regex = match Regex::new(&format!("(?i){pattern}")) {
            Ok(r) => r,
            Err(e) => {
                return Err(AutomationError::Other(format!(
                    "Invalid regex pattern: {e}"
                )));
            }
        };

        (MatchType::Regex, Some(regex))
    } else {
        let match_type = MatchType::from_operator(operator);
        (match_type, None)
    };

    let unescaped_value = unescape_dsl_value(value);

    Ok(SelectorCriteria {
        attribute: attr_lower,
        value: unescaped_value,
        match_type,
        regex_obj,
    })
}

#[must_use]
pub fn match_string(
    haystack: &str,
    needle: &str,
    match_type: MatchType,
    regex_obj: Option<&Regex>,
) -> bool {
    if match_type == MatchType::Regex {
        regex_obj.is_some_and(|re| re.is_match(haystack))
    } else {
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

#[must_use]
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

#[must_use]
pub fn control_matches_criteria(
    text: &str,
    class_name: &str,
    criteria: &[SelectorCriteria],
) -> bool {
    criteria.iter().all(|c| match c.attribute.as_str() {
        "text" => match_string(text, &c.value, c.match_type, None),
        "class" => match_string(class_name, &c.value, c.match_type, None),
        _ => false,
    })
}

pub fn window_to_selector(element: &Element) -> Result<String, AutomationError> {
    if element.element_type != ElementType::Window {
        return Err(AutomationError::Other(
            "Element must be a window to generate window selector".to_string(),
        ));
    }

    let title_empty = element.text.is_empty();
    let class_empty = element.class_name.is_empty();

    if title_empty && class_empty {
        return Err(AutomationError::Other(
            "Cannot generate window selector: both title and class are empty".to_string(),
        ));
    }

    let mut criteria = Vec::new();

    if !title_empty {
        let escaped_title = escape_dsl_value(&element.text);
        criteria.push(format!("title~{escaped_title}"));
    }

    if !class_empty {
        let escaped_class = escape_dsl_value(&element.class_name);
        criteria.push(format!("class~{escaped_class}"));
    }

    let criteria_str = criteria.join(";");
    Ok(format!("Window>{criteria_str}"))
}

pub fn control_to_selector(
    element: &Element,
    _parent: &Element,
) -> Result<String, AutomationError> {
    if element.element_type != ElementType::Control {
        return Err(AutomationError::Other(
            "Element must be a control to generate control selector".to_string(),
        ));
    }

    let text_empty = element.text.is_empty();
    let class_empty = element.class_name.is_empty();

    if text_empty && class_empty {
        return Err(AutomationError::Other(
            "Cannot generate control selector: both text and class are empty".to_string(),
        ));
    }

    let mut criteria = Vec::new();

    if !text_empty {
        let escaped_text = escape_dsl_value(&element.text);
        criteria.push(format!("text~{escaped_text}"));
    }

    if !class_empty {
        let escaped_class = escape_dsl_value(&element.class_name);
        criteria.push(format!("class~{escaped_class}"));
    }

    let criteria_str = criteria.join(";");
    Ok(format!("Control>{criteria_str}"))
}
