use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireMessage {
    segments: Vec<WireSegment>,
}

impl WireMessage {
    pub fn segments(&self) -> &[WireSegment] {
        &self.segments
    }

    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireSegment {
    fields: Vec<WireField>,
}

impl WireSegment {
    pub fn fields(&self) -> &[WireField] {
        &self.fields
    }

    pub fn code(&self) -> Option<&str> {
        self.header_component(0)
    }

    pub fn sequence(&self) -> Option<&str> {
        self.header_component(1)
    }

    pub fn version(&self) -> Option<&str> {
        self.header_component(2)
    }

    fn header_component(&self, index: usize) -> Option<&str> {
        self.fields
            .first()
            .and_then(|field| field.components().get(index))
            .map(String::as_str)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireField {
    components: Vec<String>,
}

impl WireField {
    pub fn components(&self) -> &[String] {
        &self.components
    }

    pub fn as_value(&self) -> Option<&str> {
        match self.components.as_slice() {
            [value] => Some(value),
            _ => None,
        }
    }

    pub fn value(&self) -> Option<&str> {
        self.as_value()
    }
}

pub fn parse_wire_message(input: &str) -> HbciResult<WireMessage> {
    let mut parser = WireParser::new(input);
    parser.parse()
}

struct WireParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> WireParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse(&mut self) -> HbciResult<WireMessage> {
        let mut segments = Vec::new();

        while self.position < self.input.len() {
            if self.remaining().trim().is_empty() {
                break;
            }
            segments.push(self.parse_segment()?);
        }

        Ok(WireMessage { segments })
    }

    fn parse_segment(&mut self) -> HbciResult<WireSegment> {
        let mut fields = Vec::new();
        let mut components = Vec::new();

        loop {
            let (token, delimiter) = self.read_token()?;
            components.push(token);

            match delimiter {
                Some(b':') => {}
                Some(b'+') => {
                    fields.push(WireField { components });
                    components = Vec::new();
                }
                Some(b'\'') => {
                    fields.push(WireField { components });
                    break;
                }
                None => {
                    return Err(HbciError::new(
                        HbciErrorKind::Protocol,
                        "unterminated FinTS segment",
                    ));
                }
                Some(delimiter) => {
                    return Err(HbciError::new(
                        HbciErrorKind::Protocol,
                        format!("unexpected FinTS delimiter: {}", delimiter as char),
                    ));
                }
            }
        }

        Ok(WireSegment { fields })
    }

    fn read_token(&mut self) -> HbciResult<(String, Option<u8>)> {
        let mut token = String::new();

        while self.position < self.input.len() {
            match self.current_byte() {
                b'\'' | b'+' | b':' => {
                    let delimiter = self.current_byte();
                    self.position += 1;
                    return Ok((token, Some(delimiter)));
                }
                b'?' => {
                    self.position += 1;
                    token.push(self.consume_quoted_character()?);
                }
                b'@' => token.push_str(&self.consume_binary_block()?),
                _ => token.push(self.consume_character()),
            }
        }

        Ok((token, None))
    }

    fn consume_quoted_character(&mut self) -> HbciResult<char> {
        let Some(character) = self.remaining().chars().next() else {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                "unterminated FinTS quoted character",
            ));
        };
        self.position += character.len_utf8();
        Ok(character)
    }

    fn consume_binary_block(&mut self) -> HbciResult<String> {
        let block_start = self.position;
        self.position += 1;
        let length_start = self.position;

        while self.position < self.input.len() && self.current_byte() != b'@' {
            if !self.current_byte().is_ascii_digit() {
                return Err(HbciError::new(
                    HbciErrorKind::Protocol,
                    "invalid FinTS binary length",
                ));
            }
            self.position += 1;
        }

        if self.position == self.input.len() {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                "unterminated FinTS binary length",
            ));
        }

        let length = self.input[length_start..self.position]
            .parse::<usize>()
            .map_err(|err| {
                HbciError::with_source(HbciErrorKind::Protocol, "invalid FinTS binary length", err)
            })?;
        self.position += 1;

        let block_end = self.position + length;
        if block_end > self.input.len() {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                "truncated FinTS binary payload",
            ));
        }
        if !self.input.is_char_boundary(block_end) {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                "FinTS binary payload is not valid UTF-8 at the declared boundary",
            ));
        }

        self.position = block_end;
        Ok(self.input[block_start..block_end].to_owned())
    }

    fn consume_character(&mut self) -> char {
        let character = self
            .remaining()
            .chars()
            .next()
            .expect("position is in bounds");
        self.position += character.len_utf8();
        character
    }

    fn current_byte(&self) -> u8 {
        self.input.as_bytes()[self.position]
    }

    fn remaining(&self) -> &str {
        &self.input[self.position..]
    }
}
