use std::collections::{HashMap, HashSet};
use std::fs;

fn main() {
    let metadata_keys = extract_metadata_keys();
    let locale_dir = "crates/rpa-studio/locales";
    let mut locale_keys_map: HashMap<String, HashSet<String>> = HashMap::new();

    for entry in fs::read_dir(locale_dir).expect("Failed to read locales directory") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();

        if path.is_file() && path.extension().map(|e| e == "yml").unwrap_or(false) {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let lang = filename
                .strip_suffix(".yml")
                .unwrap_or(&filename)
                .to_string();
            let keys = load_locale_keys(path.to_str().unwrap());
            locale_keys_map.insert(lang, keys);
        }
    }

    println!("=== Localization Validation Report ===\n");

    let mut missing_any = false;

    for (lang, keys) in &locale_keys_map {
        let missing: Vec<_> = metadata_keys.difference(keys).collect();
        if !missing.is_empty() {
            missing_any = true;
            println!("✗ Missing keys in {}.yml:", lang);
            for key in &missing {
                println!("  - {}", key);
            }
            println!();
        }
    }

    if !missing_any {
        println!("✓ All required localization keys are present in all locale files.");
    }

    println!("\nSummary:");
    println!("  Metadata keys: {}", metadata_keys.len());
    for (lang, keys) in &locale_keys_map {
        println!("  {}.yml keys: {}", lang, keys.len());
    }
}

fn extract_metadata_keys() -> HashSet<String> {
    let mut keys = HashSet::new();

    let metadata_content = fs::read_to_string("crates/rpa-core/src/activity_metadata.rs")
        .expect("Failed to read activity_metadata.rs");

    for line in metadata_content.lines() {
        if (line.contains("name_key:") || line.contains("button_key:"))
            && let Some(key) = extract_key_from_line(line)
        {
            keys.insert(key);
        }
    }

    keys
}

fn extract_key_from_line(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    if start < end {
        Some(line[start + 1..end].to_string())
    } else {
        None
    }
}

fn load_locale_keys(path: &str) -> HashSet<String> {
    let mut keys = HashSet::new();

    if let Ok(content) = fs::read_to_string(path) {
        for line in content.lines() {
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim();
                if !key.starts_with('_') && !key.is_empty() {
                    keys.insert(key.to_string());
                }
            }
        }
    }

    keys
}
