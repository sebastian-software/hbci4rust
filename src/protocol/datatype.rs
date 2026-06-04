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

pub(crate) fn parse_data_element(
    type_name: &str,
    value: &str,
    constraints: DataTypeConstraints,
) -> HbciResult<String> {
    match type_name {
        "AN" | "Code" | "DTAUS" | "JN" => parse_passthrough(type_name, value, constraints),
        "ID" => parse_passthrough(
            type_name,
            value,
            DataTypeConstraints {
                max_size: Some(30),
                ..constraints
            },
        ),
        "Num" => parse_num(value, constraints),
        "Dig" => parse_dig(value, constraints),
        "Ctr" => parse_country(value),
        "Cur" => parse_currency(value),
        "Date" => parse_date(value),
        "Time" => parse_time(value),
        "Float" => parse_float(value, constraints),
        "Wrt" => parse_float(
            value,
            DataTypeConstraints {
                max_size: Some(15),
                ..constraints
            },
        ),
        "Bin" => parse_binary_data_element(value, constraints),
        _ => parse_passthrough(type_name, value, constraints),
    }
}

fn parse_passthrough(
    type_name: &str,
    value: &str,
    constraints: DataTypeConstraints,
) -> HbciResult<String> {
    check_size(type_name, value, constraints)?;
    Ok(value.to_owned())
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

fn parse_num(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    require_ascii_digits("Num", value)?;
    if value.len() != 1 && value.starts_with('0') {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Num data element value must not contain leading zeroes: {value}"),
        ));
    }
    check_size("Num", value, constraints)?;
    Ok(value.to_owned())
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

fn parse_dig(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    require_ascii_digits("Dig", value)?;
    check_size("Dig", value, constraints)?;
    Ok(value.to_owned())
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

fn parse_country(value: &str) -> HbciResult<String> {
    let name = match value {
        "280" => "DE",
        "040" => "AT",
        "250" => "FR",
        "056" => "BE",
        "100" => "BG",
        "208" => "DK",
        "246" => "FI",
        "300" => "GR",
        "826" => "GB",
        "372" => "IE",
        "352" => "IS",
        "380" => "IT",
        "392" => "JP",
        "124" => "CA",
        "191" => "HR",
        "438" => "LI",
        "442" => "LU",
        "528" => "NL",
        "578" => "NO",
        "616" => "PL",
        "620" => "PT",
        "642" => "RO",
        "643" => "RU",
        "752" => "SE",
        "756" => "CH",
        "703" => "SK",
        "705" => "SI",
        "724" => "ES",
        "203" => "CZ",
        "792" => "TR",
        "348" => "HU",
        "840" => "US",
        "978" => "EU",
        _ => {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("unknown Ctr data element country code: {value}"),
            ));
        }
    };

    Ok(name.to_owned())
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

fn parse_currency(value: &str) -> HbciResult<String> {
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

fn parse_date(value: &str) -> HbciResult<String> {
    require_ascii_digits("Date", value)?;
    if value.len() != 8 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Date data element value must use YYYYMMDD: {value}"),
        ));
    }

    let year = &value[0..4];
    let month = &value[4..6];
    let day = &value[6..8];
    let month_num = parse_bounded_usize("Date month", month, 1, 12)?;
    let day_num = parse_bounded_usize("Date day", day, 1, days_in_month(year, month_num)?)?;
    Ok(format!("{year}-{month_num:02}-{day_num:02}"))
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

fn parse_time(value: &str) -> HbciResult<String> {
    require_ascii_digits("Time", value)?;
    if value.len() != 6 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Time data element value must use HHMMSS: {value}"),
        ));
    }

    let hour = parse_bounded_usize("Time hour", &value[0..2], 0, 23)?;
    let minute = parse_bounded_usize("Time minute", &value[2..4], 0, 59)?;
    let second = parse_bounded_usize("Time second", &value[4..6], 0, 59)?;
    Ok(format!("{hour:02}:{minute:02}:{second:02}"))
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

fn parse_float(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    let parsed = value.replace(',', ".");
    if !parsed.is_empty() {
        parsed.parse::<f64>().map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::InvalidArgument,
                format!("invalid Float data element value: {value}"),
                err,
            )
        })?;
    }

    check_size("Float", value, constraints)?;
    Ok(parsed)
}

fn render_binary_data_element(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    let Some((format, data)) = value.split_at_checked(1) else {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "Bin data element value must start with B or N",
        ));
    };

    let payload = match format {
        "B" => data.to_owned(),
        "N" => render_numeric_binary_payload(data)?,
        _ => {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("Bin data element value has unsupported format: {format}"),
            ));
        }
    };

    check_size("Bin", &payload, constraints)?;
    Ok(format!("@{}@{}", payload.len(), payload))
}

fn render_numeric_binary_payload(value: &str) -> HbciResult<String> {
    let bytes = positive_decimal_to_java_big_integer_bytes(value)?;
    String::from_utf8(bytes).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Unsupported,
            "numeric Bin data element payload is not UTF-8 representable in the string-backed wire renderer",
            err,
        )
    })
}

fn positive_decimal_to_java_big_integer_bytes(value: &str) -> HbciResult<Vec<u8>> {
    require_ascii_digits("Bin", value)?;
    let mut digits: Vec<u8> = value.bytes().map(|byte| byte - b'0').collect();
    while digits.len() > 1 && digits.first() == Some(&0) {
        digits.remove(0);
    }

    if digits == [0] {
        return Ok(vec![0]);
    }

    let mut bytes = Vec::new();
    while !digits.is_empty() {
        let mut quotient = Vec::new();
        let mut carry = 0u16;
        for digit in digits {
            let value = carry * 10 + u16::from(digit);
            let quotient_digit = value / 256;
            carry = value % 256;
            if !quotient.is_empty() || quotient_digit != 0 {
                quotient.push(quotient_digit as u8);
            }
        }
        bytes.push(carry as u8);
        digits = quotient;
    }

    bytes.reverse();
    if bytes.first().is_some_and(|byte| byte & 0x80 != 0) {
        bytes.insert(0, 0);
    }
    Ok(bytes)
}

fn parse_binary_data_element(value: &str, constraints: DataTypeConstraints) -> HbciResult<String> {
    if !value.starts_with('@') {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("invalid Bin data element value: {value}"),
        ));
    }
    let Some(length_end) = value[1..].find('@').map(|index| index + 1) else {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("invalid Bin data element value: {value}"),
        ));
    };

    let length = value[1..length_end].parse::<usize>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            format!("invalid Bin data element length: {value}"),
            err,
        )
    })?;
    let payload_start = length_end + 1;
    let payload_end = payload_start + length;
    if payload_end > value.len() || !value.is_char_boundary(payload_end) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("truncated Bin data element value: {value}"),
        ));
    }

    let payload = &value[payload_start..payload_end];
    check_size("Bin", payload, constraints)?;
    Ok(payload.to_owned())
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

    #[test]
    fn renders_binary_values_like_hbci4java() {
        assert_eq!(
            render_data_element("Bin", "BA+B:C", DataTypeConstraints::default())
                .expect("binary renders"),
            "@5@A+B:C"
        );
        assert_eq!(
            render_data_element("Bin", "N258", DataTypeConstraints::default())
                .expect("numeric binary renders"),
            "@2@\u{1}\u{2}"
        );
        assert!(
            render_data_element("Bin", "N128", DataTypeConstraints::default())
                .expect_err("non-UTF-8 numeric binary is rejected")
                .message()
                .contains("not UTF-8 representable")
        );
    }

    #[test]
    fn parses_date_and_time_values() {
        assert_eq!(
            parse_data_element("Date", "20240229", DataTypeConstraints::default())
                .expect("date parses"),
            "2024-02-29"
        );
        assert_eq!(
            parse_data_element("Time", "070809", DataTypeConstraints::default())
                .expect("time parses"),
            "07:08:09"
        );

        assert!(parse_data_element("Date", "20230229", DataTypeConstraints::default()).is_err());
        assert!(parse_data_element("Time", "240000", DataTypeConstraints::default()).is_err());
    }

    #[test]
    fn parses_core_wire_datatypes() {
        assert_eq!(
            parse_data_element("Num", "7", DataTypeConstraints::default()).expect("num parses"),
            "7"
        );
        assert!(parse_data_element("Num", "007", DataTypeConstraints::default()).is_err());
        assert_eq!(
            parse_data_element("Dig", "007", DataTypeConstraints::default()).expect("dig parses"),
            "007"
        );
        assert_eq!(
            parse_data_element("Ctr", "280", DataTypeConstraints::default())
                .expect("country parses"),
            "DE"
        );
        assert_eq!(
            parse_data_element("Float", "12,3", DataTypeConstraints::default())
                .expect("float parses"),
            "12.3"
        );
        assert_eq!(
            parse_data_element("Bin", "@5@A+B:C", DataTypeConstraints::default())
                .expect("binary parses"),
            "A+B:C"
        );
    }
}
