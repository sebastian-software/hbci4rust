#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mt940Document {
    pub raw: String,
}

pub fn decode_umlauts(input: &str) -> String {
    input
        .replace('[', "Ä")
        .replace('\\', "Ö")
        .replace(']', "Ü")
        .replace('~', "ß")
}

pub fn get_one_block(input: &str) -> Option<String> {
    let endpos = find_from(input, "\r\n:20:", 1).unwrap_or(input.len());
    (endpos > 0).then(|| input[..endpos].to_owned())
}

pub fn pack_multi(input: &str) -> String {
    input.replace("\r\n", "")
}

pub fn get_multi_tag_value(input: &str, tag: &str) -> Option<String> {
    let marker = format!("?{tag}");
    let pos = find_from(input, &marker, 0)?;
    let mut searchpos = pos + 3;
    let mut endpos;

    loop {
        endpos = find_from(input, "?", searchpos);

        let Some(candidate) = endpos else {
            break;
        };

        let bytes = input.as_bytes();
        if candidate + 2 < bytes.len()
            && bytes[candidate + 1].is_ascii_digit()
            && bytes[candidate + 2].is_ascii_digit()
        {
            break;
        }

        if candidate + 2 >= bytes.len() {
            endpos = None;
            break;
        }

        searchpos = candidate + 1;
    }

    let endpos = endpos.unwrap_or(input.len());
    Some(input[pos + 3..endpos].to_owned())
}

pub fn get_tag_value(input: &str, tag: &str, counter: usize) -> Option<String> {
    let mut endpos = 0;
    let mut remaining = counter;

    loop {
        let normal_tag = format!("\r\n:{tag}:");
        let broken_tag = format!("\r\n-:{tag}:");
        let (startpos, skip_length) = match find_from(input, &normal_tag, endpos) {
            Some(startpos) => (Some(startpos), 3),
            None => (find_from(input, &broken_tag, endpos), 4),
        };

        let startpos = startpos?;

        let value_start = startpos + skip_length + tag.len() + 1;
        let ret = if let Some(next_tag_start) = find_next_tag_marker(input, value_start) {
            endpos = next_tag_start;
            input[value_start..next_tag_start].to_owned()
        } else {
            remove_final_line_noise(&input[value_start..])
        };

        if remaining == 0 {
            return Some(ret);
        }
        remaining -= 1;
    }
}

fn find_from(input: &str, pattern: &str, start: usize) -> Option<usize> {
    let needle = pattern.as_bytes();
    input
        .as_bytes()
        .get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|relative| start + relative)
}

fn find_next_tag_marker(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut index = start;

    while index + 5 <= bytes.len() {
        if bytes[index] != b'\r' || bytes.get(index + 1) != Some(&b'\n') {
            index += 1;
            continue;
        }

        let mut tag_start = index + 2;
        if bytes.get(tag_start) == Some(&b'-') {
            if bytes.get(tag_start + 1) == Some(&b':') {
                tag_start += 1;
            } else if bytes.get(tag_start + 1) == Some(&b'\r')
                && bytes.get(tag_start + 2) == Some(&b'\n')
                && bytes.get(tag_start + 3) == Some(&b':')
            {
                tag_start += 3;
            } else {
                index += 1;
                continue;
            }
        }

        if bytes.get(tag_start) != Some(&b':') {
            index += 1;
            continue;
        }

        let first_digit = tag_start + 1;
        let second_digit = tag_start + 2;
        if !bytes.get(first_digit).is_some_and(u8::is_ascii_digit)
            || !bytes.get(second_digit).is_some_and(u8::is_ascii_digit)
        {
            index += 1;
            continue;
        }

        let after_digits = tag_start + 3;
        if bytes.get(after_digits) == Some(&b':') {
            return Some(index);
        }
        if bytes.get(after_digits).is_some_and(u8::is_ascii_uppercase)
            && bytes.get(after_digits + 1) == Some(&b':')
        {
            return Some(index);
        }

        index += 1;
    }

    None
}

fn remove_final_line_noise(input: &str) -> String {
    input
        .chars()
        .filter(|character| !matches!(character, '\r' | '\n' | '-'))
        .collect()
}

impl Mt940Document {
    pub fn parse(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}
