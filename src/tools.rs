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
