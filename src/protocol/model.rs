use std::collections::BTreeMap;

use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};

use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::protocol::ProtocolSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSyntax {
    version: &'static str,
    definitions: BTreeMap<String, SyntaxDefinition>,
}

impl ProtocolSyntax {
    pub(crate) fn parse(spec: &ProtocolSpec) -> HbciResult<Self> {
        let parser = SyntaxParser::new(spec.version);
        parser.parse(spec.xml)
    }

    pub fn version(&self) -> &'static str {
        self.version
    }

    pub fn definition(&self, id: &str) -> Option<&SyntaxDefinition> {
        self.definitions.get(id)
    }

    pub fn definitions(&self) -> impl Iterator<Item = &SyntaxDefinition> {
        self.definitions.values()
    }

    pub fn definition_count(&self, kind: DefinitionKind) -> usize {
        self.definitions
            .values()
            .filter(|definition| definition.kind == kind)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDefinition {
    pub id: String,
    pub kind: DefinitionKind,
    pub needs_request_tag: bool,
    pub dont_sign: bool,
    pub dont_crypt: bool,
    pub children: Vec<SyntaxChild>,
    pub values: Vec<SyntaxValue>,
    pub valids: Vec<SyntaxValidSet>,
}

impl SyntaxDefinition {
    fn new(kind: DefinitionKind, id: String) -> Self {
        Self {
            id,
            kind,
            needs_request_tag: false,
            dont_sign: false,
            dont_crypt: false,
            children: Vec::new(),
            values: Vec::new(),
            valids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefinitionKind {
    Deg,
    Seg,
    Sf,
    Msg,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxChild {
    pub kind: SyntaxChildKind,
    pub type_name: String,
    pub name: Option<String>,
    pub min_size: Option<String>,
    pub max_size: Option<String>,
    pub min_num: Option<String>,
    pub max_num: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxChildKind {
    De,
    Deg,
    Seg,
    Sf,
    EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxValue {
    pub path: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxValidSet {
    pub path: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TextCapture {
    Value { path: String, text: String },
    ValidValue(String),
}

struct SyntaxParser {
    version: &'static str,
    definitions: BTreeMap<String, SyntaxDefinition>,
    current_definition: Option<SyntaxDefinition>,
    current_valids: Option<SyntaxValidSet>,
    text_capture: Option<TextCapture>,
}

impl SyntaxParser {
    fn new(version: &'static str) -> Self {
        Self {
            version,
            definitions: BTreeMap::new(),
            current_definition: None,
            current_valids: None,
            text_capture: None,
        }
    }

    fn parse(mut self, xml: &str) -> HbciResult<ProtocolSyntax> {
        let mut reader = quick_xml::Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        loop {
            match reader.read_event() {
                Ok(Event::Start(event)) => self.handle_start(&reader, event)?,
                Ok(Event::Empty(event)) => self.handle_empty(&reader, event)?,
                Ok(Event::Text(event)) => {
                    if let Some(capture) = &mut self.text_capture {
                        let text = event.decode().map_err(|err| {
                            HbciError::with_source(
                                HbciErrorKind::Protocol,
                                "failed to decode protocol text",
                                err,
                            )
                        })?;
                        match capture {
                            TextCapture::Value { text: target, .. }
                            | TextCapture::ValidValue(target) => target.push_str(&text),
                        }
                    }
                }
                Ok(Event::GeneralRef(entity)) => {
                    if let Some(definition) = &mut self.current_definition {
                        let type_name = entity.decode().map_err(|err| {
                            HbciError::with_source(
                                HbciErrorKind::Protocol,
                                "failed to decode protocol entity reference",
                                err,
                            )
                        })?;
                        definition.children.push(SyntaxChild {
                            kind: SyntaxChildKind::EntityRef,
                            type_name: type_name.into_owned(),
                            name: None,
                            min_size: None,
                            max_size: None,
                            min_num: None,
                            max_num: None,
                        });
                    }
                }
                Ok(Event::End(event)) => self.handle_end(event.name().as_ref())?,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(err) => {
                    return Err(HbciError::with_source(
                        HbciErrorKind::Protocol,
                        format!("failed to parse HBCI {} protocol syntax", self.version),
                        err,
                    ));
                }
            }
        }

        Ok(ProtocolSyntax {
            version: self.version,
            definitions: self.definitions,
        })
    }

    fn handle_start(
        &mut self,
        reader: &quick_xml::Reader<&[u8]>,
        event: BytesStart<'_>,
    ) -> HbciResult<()> {
        match event.name().as_ref() {
            b"DEGdef" => self.start_definition(reader, event, DefinitionKind::Deg),
            b"SEGdef" => self.start_definition(reader, event, DefinitionKind::Seg),
            b"SFdef" => self.start_definition(reader, event, DefinitionKind::Sf),
            b"MSGdef" => self.start_definition(reader, event, DefinitionKind::Msg),
            b"valids" => {
                let path = required_attr(reader, &event, b"path")?;
                self.current_valids = Some(SyntaxValidSet {
                    path,
                    values: Vec::new(),
                });
                Ok(())
            }
            b"validvalue" => {
                self.text_capture = Some(TextCapture::ValidValue(String::new()));
                Ok(())
            }
            b"value" => {
                let path = required_attr(reader, &event, b"path")?;
                self.text_capture = Some(TextCapture::Value {
                    path,
                    text: String::new(),
                });
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_empty(
        &mut self,
        reader: &quick_xml::Reader<&[u8]>,
        event: BytesStart<'_>,
    ) -> HbciResult<()> {
        let kind = match event.name().as_ref() {
            b"DE" => SyntaxChildKind::De,
            b"DEG" => SyntaxChildKind::Deg,
            b"SEG" => SyntaxChildKind::Seg,
            b"SF" => SyntaxChildKind::Sf,
            _ => return Ok(()),
        };

        let Some(definition) = &mut self.current_definition else {
            return Ok(());
        };

        definition.children.push(parse_child(reader, &event, kind)?);
        Ok(())
    }

    fn handle_end(&mut self, name: &[u8]) -> HbciResult<()> {
        match name {
            b"DEGdef" | b"SEGdef" | b"SFdef" | b"MSGdef" => {
                let definition = self.current_definition.take().ok_or_else(|| {
                    HbciError::new(
                        HbciErrorKind::Protocol,
                        "protocol definition stack underflow",
                    )
                })?;
                self.definitions.insert(definition.id.clone(), definition);
            }
            b"valids" => {
                let valids = self.current_valids.take().ok_or_else(|| {
                    HbciError::new(HbciErrorKind::Protocol, "protocol valids stack underflow")
                })?;
                if let Some(definition) = &mut self.current_definition {
                    definition.valids.push(valids);
                }
            }
            b"validvalue" => {
                let Some(TextCapture::ValidValue(text)) = self.text_capture.take() else {
                    return Ok(());
                };
                if let Some(valids) = &mut self.current_valids {
                    valids.values.push(text);
                }
            }
            b"value" => {
                let Some(TextCapture::Value { path, text }) = self.text_capture.take() else {
                    return Ok(());
                };
                if let Some(definition) = &mut self.current_definition {
                    definition.values.push(SyntaxValue { path, value: text });
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn start_definition(
        &mut self,
        reader: &quick_xml::Reader<&[u8]>,
        event: BytesStart<'_>,
        kind: DefinitionKind,
    ) -> HbciResult<()> {
        let id = required_attr(reader, &event, b"id")?;
        let mut definition = SyntaxDefinition::new(kind, id);
        definition.needs_request_tag =
            attr(reader, &event, b"needsRequestTag")?.as_deref() == Some("1");
        definition.dont_sign = attr(reader, &event, b"dontsign")?.as_deref() == Some("1");
        definition.dont_crypt = attr(reader, &event, b"dontcrypt")?.as_deref() == Some("1");
        self.current_definition = Some(definition);
        Ok(())
    }
}

fn parse_child(
    reader: &quick_xml::Reader<&[u8]>,
    event: &BytesStart<'_>,
    kind: SyntaxChildKind,
) -> HbciResult<SyntaxChild> {
    let type_attr = attr(reader, event, b"type")?;
    let name_attr = attr(reader, event, b"name")?;
    let type_name = type_attr
        .clone()
        .or_else(|| name_attr.clone())
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::Protocol,
                "protocol child is missing type/name attribute",
            )
        })?;

    Ok(SyntaxChild {
        kind,
        type_name,
        name: name_attr,
        min_size: attr(reader, event, b"minsize")?,
        max_size: attr(reader, event, b"maxsize")?,
        min_num: attr(reader, event, b"minnum")?,
        max_num: attr(reader, event, b"maxnum")?,
    })
}

fn required_attr(
    reader: &quick_xml::Reader<&[u8]>,
    event: &BytesStart<'_>,
    name: &[u8],
) -> HbciResult<String> {
    attr(reader, event, name)?.ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::Protocol,
            format!(
                "protocol element {} is missing required {} attribute",
                String::from_utf8_lossy(event.name().as_ref()),
                String::from_utf8_lossy(name)
            ),
        )
    })
}

fn attr(
    reader: &quick_xml::Reader<&[u8]>,
    event: &BytesStart<'_>,
    name: &[u8],
) -> HbciResult<Option<String>> {
    for attr in event.attributes().with_checks(false) {
        let attr = attr.map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::Protocol,
                "failed to parse XML attribute",
                err,
            )
        })?;
        if attr.key.as_ref() == name {
            let value = attr
                .decoded_and_normalized_value(XmlVersion::Explicit1_0, reader.decoder())
                .map_err(|err| {
                    HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to decode XML attribute",
                        err,
                    )
                })?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}
