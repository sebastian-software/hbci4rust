use std::path::{Path, PathBuf};

pub fn to_ins_code(hbci_code: Option<&str>) -> Option<String> {
    let hbci_code = hbci_code?;
    if hbci_code.chars().count() < 3 {
        return Some(hbci_code.to_owned());
    }

    Some(
        hbci_code
            .chars()
            .enumerate()
            .map(|(index, character)| if index == 1 { 'I' } else { character })
            .collect(),
    )
}

pub fn to_parameter_code(hbci_code: Option<&str>) -> Option<String> {
    to_ins_code(hbci_code).map(|code| format!("{code}S"))
}

pub fn to_boolean(value: Option<&str>) -> bool {
    value.is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

pub fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

pub fn join_strings(values: Option<&[&str]>, separator: Option<&str>) -> Option<String> {
    let values = values?;
    let mut result = String::new();
    let mut first = true;

    for value in values {
        if !first && let Some(separator) = separator {
            result.push_str(separator);
        }
        first = false;
        result.push_str(value);
    }

    Some(result)
}

pub fn safe_filename(filename: Option<&str>) -> Option<String> {
    let filename = filename?;
    if filename.is_empty() {
        return Some(String::new());
    }

    let absolute = absolute_path_for(filename);
    let original_name = absolute
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let safe_name = sanitize_filename_component(original_name);
    let new_path = match absolute.parent() {
        Some(parent) => parent.join(safe_name),
        None => PathBuf::from(safe_name),
    };
    let new_name = new_path.to_string_lossy().into_owned();

    if new_name == filename {
        Some(filename.to_owned())
    } else {
        Some(new_name)
    }
}

fn absolute_path_for(filename: &str) -> PathBuf {
    let path = Path::new(filename);
    if path.is_absolute() {
        return path.to_path_buf();
    }

    std::env::current_dir()
        .map(|cwd| cwd.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn sanitize_filename_component(name: &str) -> String {
    name.chars()
        .filter(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
        .take(25)
        .collect()
}
