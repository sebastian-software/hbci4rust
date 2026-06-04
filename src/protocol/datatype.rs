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
        "Date" => render_date(value),
        "Time" => render_time(value),
        "Float" => render_float(value, constraints),
        "Wrt" => render_float(
            value,
            DataTypeConstraints {
                max_size: Some(15),
                ..constraints
            },
        ),
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

fn render_date(value: &str) -> HbciResult<String> {
    let value = value.trim();
    let Some((year, month_day)) = value.split_once('-') else {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Date data element value must use YYYY-MM-DD: {value}"),
        ));
    };
    let Some((month, day)) = month_day.split_once('-') else {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Date data element value must use YYYY-MM-DD: {value}"),
        ));
    };
    require_ascii_digits("Date", year)?;
    require_ascii_digits("Date", month)?;
    require_ascii_digits("Date", day)?;

    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Date data element value must use YYYY-MM-DD: {value}"),
        ));
    }

    let month_num = parse_bounded_usize("Date month", month, 1, 12)?;
    let day_num = parse_bounded_usize("Date day", day, 1, days_in_month(year, month_num)?)?;
    Ok(format!("{year}{month_num:02}{day_num:02}"))
}

fn render_time(value: &str) -> HbciResult<String> {
    let value = value.trim();
    let parts: Vec<_> = value.split(':').collect();
    if parts.len() != 3 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Time data element value must use HH:MM:SS: {value}"),
        ));
    }

    let hour = parse_bounded_usize("Time hour", parts[0], 0, 23)?;
    let minute = parse_bounded_usize("Time minute", parts[1], 0, 59)?;
    let second = parse_bounded_usize("Time second", parts[2], 0, 59)?;
    Ok(format!("{hour:02}{minute:02}{second:02}"))
}

fn render_float(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    let value = value.trim().replace(',', ".");
    let parsed = value.parse::<f64>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            format!("invalid Float data element value: {value}"),
            err,
        )
    })?;

    if !parsed.is_finite() {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("invalid Float data element value: {value}"),
        ));
    }

    let rounded = (parsed * 100.0).round() / 100.0;
    let mut rendered = format!("{rounded:.2}");
    while rendered.ends_with('0') {
        rendered.pop();
    }
    let rendered = rendered.replace('.', ",");

    check_size("Float", &rendered, constraints)?;
    Ok(rendered)
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

fn parse_bounded_usize(
    type_name: &str,
    value: &str,
    min_value: usize,
    max_value: usize,
) -> HbciResult<usize> {
    require_ascii_digits(type_name, value)?;
    let parsed = value.parse::<usize>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            format!("{type_name} data element value is invalid: {value}"),
            err,
        )
    })?;
    if parsed < min_value || parsed > max_value {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("{type_name} data element value is out of range: {value}"),
        ));
    }
    Ok(parsed)
}

fn days_in_month(year: &str, month: usize) -> HbciResult<usize> {
    let year = year.parse::<usize>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            format!("Date year data element value is invalid: {year}"),
            err,
        )
    })?;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("Date month data element value is out of range: {month}"),
            ));
        }
    };
    Ok(days)
}

fn is_leap_year(year: usize) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_date_and_time_values() {
        assert_eq!(
            render_data_element("Date", "2024-02-29", DataTypeConstraints::default())
                .expect("date renders"),
            "20240229"
        );
        assert_eq!(
            render_data_element("Time", "07:08:09", DataTypeConstraints::default())
                .expect("time renders"),
            "070809"
        );

        assert!(render_data_element("Date", "2023-02-29", DataTypeConstraints::default()).is_err());
        assert!(render_data_element("Time", "24:00:00", DataTypeConstraints::default()).is_err());
    }

    #[test]
    fn renders_float_and_wrt_values() {
        assert_eq!(
            render_data_element("Float", "1", DataTypeConstraints::default())
                .expect("float renders"),
            "1,"
        );
        assert_eq!(
            render_data_element("Float", "12.30", DataTypeConstraints::default())
                .expect("float renders"),
            "12,3"
        );
        assert_eq!(
            render_data_element("Wrt", "12,30", DataTypeConstraints::default())
                .expect("amount renders"),
            "12,3"
        );

        assert!(
            render_data_element("Wrt", "123456789012345.6", DataTypeConstraints::default())
                .is_err()
        );
    }
}
