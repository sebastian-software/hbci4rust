use std::collections::BTreeMap;

use super::datatype::{DataTypeConstraints, parse_data_element};
use super::{DefinitionKind, ProtocolSyntax, SyntaxChild, SyntaxChildKind, SyntaxDefinition};
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

    pub fn resolve_segments<'syntax, 'wire>(
        &'wire self,
        syntax: &'syntax ProtocolSyntax,
    ) -> HbciResult<ResolvedWireMessage<'syntax, 'wire>> {
        let mut segments = Vec::new();

        for segment in &self.segments {
            let code = segment.code().ok_or_else(|| {
                HbciError::new(HbciErrorKind::Protocol, "FinTS segment header has no code")
            })?;
            let version = segment.version().ok_or_else(|| {
                HbciError::new(
                    HbciErrorKind::Protocol,
                    format!("FinTS segment {code} header has no version"),
                )
            })?;
            let definition = syntax.segment_definition(code, version).ok_or_else(|| {
                HbciError::new(
                    HbciErrorKind::Protocol,
                    format!(
                        "FinTS segment {code}:{version} is not defined in HBCI {} syntax",
                        syntax.version()
                    ),
                )
            })?;

            segments.push(ResolvedWireSegment {
                wire_segment: segment,
                definition,
            });
        }

        Ok(ResolvedWireMessage { segments })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedWireMessage<'syntax, 'wire> {
    segments: Vec<ResolvedWireSegment<'syntax, 'wire>>,
}

impl<'syntax, 'wire> ResolvedWireMessage<'syntax, 'wire> {
    pub fn segments(&self) -> &[ResolvedWireSegment<'syntax, 'wire>] {
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
pub struct ResolvedWireSegment<'syntax, 'wire> {
    wire_segment: &'wire WireSegment,
    definition: &'syntax SyntaxDefinition,
}

impl<'syntax, 'wire> ResolvedWireSegment<'syntax, 'wire> {
    pub fn wire_segment(&self) -> &'wire WireSegment {
        self.wire_segment
    }

    pub fn definition(&self) -> &'syntax SyntaxDefinition {
        self.definition
    }

    pub fn code(&self) -> Option<&str> {
        self.wire_segment.code()
    }

    pub fn sequence(&self) -> Option<&str> {
        self.wire_segment.sequence()
    }

    pub fn version(&self) -> Option<&str> {
        self.wire_segment.version()
    }

    pub fn values(&self, syntax: &ProtocolSyntax) -> HbciResult<BTreeMap<String, String>> {
        let mut values = BTreeMap::new();
        let mut cursor = FieldCursor::new(self.wire_segment.fields());
        collect_fields(
            syntax,
            self.definition.children.as_slice(),
            &self.definition.id,
            &mut cursor,
            &mut values,
        )?;
        Ok(values)
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

fn collect_fields(
    syntax: &ProtocolSyntax,
    children: &[SyntaxChild],
    parent_path: &str,
    cursor: &mut FieldCursor<'_>,
    values: &mut BTreeMap<String, String>,
) -> HbciResult<()> {
    for (child_index, child) in children.iter().enumerate() {
        let min_num = occurrence_min(child)?;
        let max_num = occurrence_max(child)?;
        let min_rest = minimum_field_slots(&children[child_index + 1..])?;
        let mut occurrence_index = 0;

        while occurrence_index < max_num
            && cursor.remaining() > 0
            && (occurrence_index < min_num || cursor.remaining() > min_rest)
        {
            let path = child_path(parent_path, child, occurrence_index);
            let field = cursor.next().expect("remaining field exists");
            collect_field_child(syntax, child, &path, field, values)?;
            occurrence_index += 1;
        }

        if occurrence_index < min_num {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!(
                    "FinTS segment field {} is missing required value at {parent_path}",
                    child_display_name(child),
                ),
            ));
        }
    }

    if cursor.remaining() != 0 {
        return Err(HbciError::new(
            HbciErrorKind::Protocol,
            format!("{parent_path} has trailing FinTS fields"),
        ));
    }

    Ok(())
}

fn collect_field_child(
    syntax: &ProtocolSyntax,
    child: &SyntaxChild,
    path: &str,
    field: &WireField,
    values: &mut BTreeMap<String, String>,
) -> HbciResult<()> {
    match child.kind {
        SyntaxChildKind::De => {
            let Some(value) = field.value() else {
                return Err(HbciError::new(
                    HbciErrorKind::Protocol,
                    format!("{path} expected one FinTS field value"),
                ));
            };
            insert_data_element_value(values, path, child, value)
        }
        SyntaxChildKind::Deg => {
            let definition = referenced_definition(syntax, child, DefinitionKind::Deg)?;
            let mut cursor = ComponentCursor::new(field.components());
            collect_components(
                syntax,
                definition.children.as_slice(),
                path,
                &mut cursor,
                values,
            )?;
            if cursor.remaining() != 0 {
                return Err(HbciError::new(
                    HbciErrorKind::Protocol,
                    format!("{path} has trailing FinTS data-element components"),
                ));
            }
            Ok(())
        }
        SyntaxChildKind::Seg | SyntaxChildKind::Sf | SyntaxChildKind::EntityRef => {
            Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!(
                    "unsupported FinTS field child {} in {path}",
                    child_display_name(child),
                ),
            ))
        }
    }
}

fn collect_components(
    syntax: &ProtocolSyntax,
    children: &[SyntaxChild],
    parent_path: &str,
    cursor: &mut ComponentCursor<'_>,
    values: &mut BTreeMap<String, String>,
) -> HbciResult<()> {
    for (child_index, child) in children.iter().enumerate() {
        let min_num = occurrence_min(child)?;
        let max_num = occurrence_max(child)?;
        let min_rest = minimum_component_slots(syntax, &children[child_index + 1..])?;
        let mut occurrence_index = 0;

        while occurrence_index < max_num
            && cursor.remaining() > 0
            && (occurrence_index < min_num || cursor.remaining() > min_rest)
        {
            let path = child_path(parent_path, child, occurrence_index);
            collect_component_child(syntax, child, &path, cursor, values)?;
            occurrence_index += 1;
        }

        if occurrence_index < min_num {
            return Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!(
                    "FinTS data-element component {} is missing required value at {parent_path}",
                    child_display_name(child),
                ),
            ));
        }
    }

    Ok(())
}

fn collect_component_child(
    syntax: &ProtocolSyntax,
    child: &SyntaxChild,
    path: &str,
    cursor: &mut ComponentCursor<'_>,
    values: &mut BTreeMap<String, String>,
) -> HbciResult<()> {
    match child.kind {
        SyntaxChildKind::De => {
            let value = cursor.next().ok_or_else(|| {
                HbciError::new(
                    HbciErrorKind::Protocol,
                    format!("{path} expected a FinTS data-element component"),
                )
            })?;
            insert_data_element_value(values, path, child, value)
        }
        SyntaxChildKind::Deg => {
            let definition = referenced_definition(syntax, child, DefinitionKind::Deg)?;
            collect_components(syntax, definition.children.as_slice(), path, cursor, values)
        }
        SyntaxChildKind::Seg | SyntaxChildKind::Sf | SyntaxChildKind::EntityRef => {
            Err(HbciError::new(
                HbciErrorKind::Protocol,
                format!(
                    "unsupported FinTS component child {} in {path}",
                    child_display_name(child),
                ),
            ))
        }
    }
}

fn minimum_field_slots(children: &[SyntaxChild]) -> HbciResult<usize> {
    children
        .iter()
        .map(occurrence_min)
        .try_fold(0usize, |total, min_num| {
            min_num.map(|min_num| total.saturating_add(min_num))
        })
}

fn minimum_component_slots(syntax: &ProtocolSyntax, children: &[SyntaxChild]) -> HbciResult<usize> {
    let mut total = 0usize;

    for child in children {
        let min_num = occurrence_min(child)?;
        let child_min = match child.kind {
            SyntaxChildKind::De => 1,
            SyntaxChildKind::Deg => {
                let definition = referenced_definition(syntax, child, DefinitionKind::Deg)?;
                minimum_component_slots(syntax, definition.children.as_slice())?
            }
            SyntaxChildKind::Seg | SyntaxChildKind::Sf | SyntaxChildKind::EntityRef => 0,
        };
        total = total.saturating_add(min_num.saturating_mul(child_min));
    }

    Ok(total)
}

fn referenced_definition<'a>(
    syntax: &'a ProtocolSyntax,
    child: &SyntaxChild,
    expected_kind: DefinitionKind,
) -> HbciResult<&'a SyntaxDefinition> {
    let definition = syntax.definition(&child.type_name).ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::Protocol,
            format!(
                "protocol definition {} is not defined",
                child_display_name(child),
            ),
        )
    })?;

    if definition.kind != expected_kind {
        return Err(HbciError::new(
            HbciErrorKind::Protocol,
            format!(
                "protocol definition {} has unexpected kind",
                child_display_name(child),
            ),
        ));
    }

    Ok(definition)
}

fn occurrence_min(child: &SyntaxChild) -> HbciResult<usize> {
    parse_occurrence(&child.min_num, 1, "minnum", child)
}

fn occurrence_max(child: &SyntaxChild) -> HbciResult<usize> {
    match parse_occurrence(&child.max_num, 1, "maxnum", child)? {
        0 => Ok(usize::MAX),
        value => Ok(value),
    }
}

fn parse_occurrence(
    value: &Option<String>,
    default_value: usize,
    attribute_name: &str,
    child: &SyntaxChild,
) -> HbciResult<usize> {
    let Some(value) = value else {
        return Ok(default_value);
    };

    value.parse::<usize>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            format!(
                "invalid {attribute_name} on protocol child {}",
                child_display_name(child),
            ),
            err,
        )
    })
}

fn data_type_constraints(child: &SyntaxChild) -> HbciResult<DataTypeConstraints> {
    Ok(DataTypeConstraints {
        min_size: Some(parse_size(&child.min_size, 1, "minsize", child)?),
        max_size: Some(parse_size(&child.max_size, 0, "maxsize", child)?),
    })
}

fn parse_size(
    value: &Option<String>,
    default_value: usize,
    attribute_name: &str,
    child: &SyntaxChild,
) -> HbciResult<usize> {
    let Some(value) = value else {
        return Ok(default_value);
    };

    value.parse::<usize>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            format!(
                "invalid {attribute_name} on protocol child {}",
                child_display_name(child),
            ),
            err,
        )
    })
}

fn child_path(parent_path: &str, child: &SyntaxChild, occurrence_index: usize) -> String {
    let name = child_name(child);
    if occurrence_index == 0 {
        format!("{parent_path}.{name}")
    } else {
        format!("{parent_path}.{name}_{}", occurrence_index + 1)
    }
}

fn child_name(child: &SyntaxChild) -> &str {
    child.name.as_deref().unwrap_or(&child.type_name)
}

fn child_display_name(child: &SyntaxChild) -> &str {
    child.name.as_deref().unwrap_or(&child.type_name)
}

fn insert_data_element_value(
    values: &mut BTreeMap<String, String>,
    path: &str,
    child: &SyntaxChild,
    value: &str,
) -> HbciResult<()> {
    let parsed_value = if value.is_empty() {
        String::new()
    } else {
        parse_data_element(&child.type_name, value, data_type_constraints(child)?)?
    };

    if values.insert(path.to_owned(), parsed_value).is_some() {
        return Err(HbciError::new(
            HbciErrorKind::Protocol,
            format!("FinTS value path {path} was parsed more than once"),
        ));
    }
    Ok(())
}

struct FieldCursor<'a> {
    fields: &'a [WireField],
    index: usize,
}

impl<'a> FieldCursor<'a> {
    fn new(fields: &'a [WireField]) -> Self {
        Self { fields, index: 0 }
    }

    fn remaining(&self) -> usize {
        self.fields.len().saturating_sub(self.index)
    }

    fn next(&mut self) -> Option<&'a WireField> {
        let field = self.fields.get(self.index)?;
        self.index += 1;
        Some(field)
    }
}

struct ComponentCursor<'a> {
    components: &'a [String],
    index: usize,
}

impl<'a> ComponentCursor<'a> {
    fn new(components: &'a [String]) -> Self {
        Self {
            components,
            index: 0,
        }
    }

    fn remaining(&self) -> usize {
        self.components.len().saturating_sub(self.index)
    }

    fn next(&mut self) -> Option<&'a str> {
        let component = self.components.get(self.index)?;
        self.index += 1;
        Some(component)
    }
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
