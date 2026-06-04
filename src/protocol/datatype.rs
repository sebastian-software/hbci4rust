use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DataTypeConstraints {
    pub min_size: Option<usize>,
    pub max_size: Option<usize>,
}

pub(crate) fn render_data_element(
    type_name: &str,
    value: &str,
    constraints: DataTypeConstraints,
) -> HbciResult<String> {
    match type_name {
        "AN" | "Code" | "DTAUS" | "JN" => {
            let value = value.trim();
            check_size(type_name, value, constraints)?;
            Ok(quote_data_element(value))
        }
        "ID" => {
            let value = value.trim();
            check_size(
                type_name,
                value,
                DataTypeConstraints {
                    max_size: Some(30),
                    ..constraints
                },
            )?;
            Ok(quote_data_element(value))
        }
        "Num" => render_num(value, constraints),
        "Dig" => render_dig(value, constraints),
        "Ctr" => render_country(value),
        "Cur" => render_currency(value),
        "Bin" => render_binary_data_element(value, constraints),
        _ => {
            check_size(type_name, value, constraints)?;
            Ok(quote_data_element(value))
        }
    }
}

fn render_num(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    value.parse::<i64>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            format!("invalid Num data element value: {value}"),
            err,
        )
    })?;

    let mut rendered = value.to_owned();
    while rendered.len() != 1 && rendered.starts_with('0') {
        rendered.remove(0);
    }

    check_size("Num", &rendered, constraints)?;
    Ok(rendered)
}

fn render_dig(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    let value = value.trim();
    require_ascii_digits("Dig", value)?;

    let min_size = constraints.min_size.unwrap_or(1);
    let rendered = if value.len() < min_size {
        format!("{value:0>min_size$}")
    } else {
        value.to_owned()
    };

    check_size("Dig", &rendered, constraints)?;
    Ok(rendered)
}

fn render_country(value: &str) -> HbciResult<String> {
    let value = value.trim();
    let code = match value {
        "DE" => "280",
        "AT" => "040",
        "FR" => "250",
        "BE" => "056",
        "BG" => "100",
        "DK" => "208",
        "FI" => "246",
        "GR" => "300",
        "GB" => "826",
        "IE" => "372",
        "IS" => "352",
        "IT" => "380",
        "JP" => "392",
        "CA" => "124",
        "HR" => "191",
        "LI" => "438",
        "LU" => "442",
        "NL" => "528",
        "NO" => "578",
        "PL" => "616",
        "PT" => "620",
        "RO" => "642",
        "RU" => "643",
        "SE" => "752",
        "CH" => "756",
        "SK" => "703",
        "SI" => "705",
        "ES" => "724",
        "CZ" => "203",
        "TR" => "792",
        "HU" => "348",
        "US" => "840",
        "EU" => "978",
        _ => {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("unknown Ctr data element country: {value}"),
            ));
        }
    };

    Ok(code.to_owned())
}

fn render_currency(value: &str) -> HbciResult<String> {
    let value = value.trim();
    check_size(
        "Cur",
        value,
        DataTypeConstraints {
            min_size: Some(3),
            max_size: Some(3),
        },
    )?;
    Ok(value.to_owned())
}

fn render_binary_data_element(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    let Some(payload) = value.strip_prefix('B') else {
        return Err(HbciError::new(
            HbciErrorKind::Unsupported,
            "numeric binary data element rendering is not ported yet",
        ));
    };

    check_size("Bin", payload, constraints)?;
    Ok(format!("@{}@{}", payload.len(), payload))
}

fn require_ascii_digits(type_name: &str, value: &str) -> HbciResult<()> {
    if value.chars().all(|character| character.is_ascii_digit()) {
        Ok(())
    } else {
        Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("{type_name} data element value must contain only digits: {value}"),
        ))
    }
}

fn check_size(type_name: &str, value: &str, constraints: DataTypeConstraints) -> HbciResult<()> {
    let len = value.len();
    if let Some(min_size) = constraints.min_size
        && len < min_size
    {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("{type_name} data element value is too short: {len} < {min_size} ({value})"),
        ));
    }
    if let Some(max_size) = constraints.max_size
        && max_size != 0
        && len > max_size
    {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("{type_name} data element value is too long: {len} > {max_size} ({value})"),
        ));
    }

    Ok(())
}

fn quote_data_element(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '+' | ':' | '\'' | '?' | '@' => quoted.push('?'),
            _ => {}
        }
        quoted.push(character);
    }
    quoted
}
