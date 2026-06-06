use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

use crate::error::{HbciError, HbciErrorKind, HbciResult};

pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATE_UNDEFINED: &str = "1999-01-01";
pub const CAMT_052_001_01_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.01";
pub const CAMT_052_001_02_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.02";
pub const CAMT_052_001_03_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.03";
pub const CAMT_052_001_04_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.04";
pub const CAMT_052_001_05_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.05";
pub const CAMT_052_001_06_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.06";
pub const CAMT_052_001_07_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.07";
pub const CAMT_052_001_08_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.08";
pub const CAMT_052_001_09_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.09";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SepaKind {
    Pain001,
    Pain008,
    Camt052,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SepaVersion {
    kind: SepaKind,
    major: u16,
    minor: u16,
    urn: &'static str,
    schema_file: Option<&'static str>,
    order: u16,
}

impl SepaVersion {
    pub const CAMT_052_001_01: Self = Self::camt(1, CAMT_052_001_01_URN, "camt.052.001.01.xsd");
    pub const CAMT_052_001_02: Self = Self::camt(2, CAMT_052_001_02_URN, "camt.052.001.02.xsd");
    pub const CAMT_052_001_03: Self = Self::camt(3, CAMT_052_001_03_URN, "camt.052.001.03.xsd");
    pub const CAMT_052_001_04: Self = Self::camt(4, CAMT_052_001_04_URN, "camt.052.001.04.xsd");
    pub const CAMT_052_001_05: Self = Self::camt(5, CAMT_052_001_05_URN, "camt.052.001.05.xsd");
    pub const CAMT_052_001_06: Self = Self::camt(6, CAMT_052_001_06_URN, "camt.052.001.06.xsd");
    pub const CAMT_052_001_07: Self = Self::camt(7, CAMT_052_001_07_URN, "camt.052.001.07.xsd");
    pub const CAMT_052_001_08: Self = Self::camt(8, CAMT_052_001_08_URN, "camt.052.001.08.xsd");
    pub const CAMT_052_001_09: Self = Self::camt(9, CAMT_052_001_09_URN, "camt.052.001.09.xsd");

    const fn camt(order: u16, urn: &'static str, schema_file: &'static str) -> Self {
        Self {
            kind: SepaKind::Camt052,
            major: 1,
            minor: order,
            urn,
            schema_file: Some(schema_file),
            order,
        }
    }

    pub const fn kind(self) -> SepaKind {
        self.kind
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn minor(self) -> u16 {
        self.minor
    }

    pub const fn urn(self) -> &'static str {
        self.urn
    }

    pub const fn schema_file(self) -> Option<&'static str> {
        self.schema_file
    }

    pub fn schema_location(self) -> Option<String> {
        self.schema_file
            .map(|schema_file| format!("{} {}", self.urn, schema_file))
    }

    pub fn known_camt_versions() -> &'static [Self] {
        &[
            Self::CAMT_052_001_01,
            Self::CAMT_052_001_02,
            Self::CAMT_052_001_03,
            Self::CAMT_052_001_04,
            Self::CAMT_052_001_05,
            Self::CAMT_052_001_06,
            Self::CAMT_052_001_07,
            Self::CAMT_052_001_08,
            Self::CAMT_052_001_09,
        ]
    }

    pub fn by_urn(urn: &str) -> Option<Self> {
        Self::known_camt_versions()
            .iter()
            .copied()
            .find(|version| version.urn == urn)
    }

    pub fn find_greatest(versions: &[Self]) -> Option<Self> {
        versions
            .iter()
            .copied()
            .max_by_key(|version| (version.kind, version.order, version.major, version.minor))
    }

    pub fn autodetect(xml: &str) -> HbciResult<Option<Self>> {
        let namespace = root_namespace(xml)?;
        Ok(namespace.and_then(|namespace| Self::by_urn(&namespace)))
    }

    pub fn choose(descriptor: Option<&str>, data: Option<&str>) -> HbciResult<Option<Self>> {
        let descriptor_version = descriptor
            .filter(|value| !value.is_empty())
            .and_then(Self::by_urn);
        let data_version = match data.filter(|value| !value.is_empty()) {
            Some(data) => Self::autodetect(data)?,
            None => None,
        };

        Ok(data_version.or(descriptor_version))
    }
}

fn root_namespace(xml: &str) -> HbciResult<Option<String>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                for attr in event.attributes() {
                    let attr = attr.map_err(|err| {
                        HbciError::with_source(
                            HbciErrorKind::Protocol,
                            "failed to read XML root attribute",
                            err,
                        )
                    })?;
                    if attr.key.as_ref() == b"xmlns" {
                        let value = attr
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .map_err(|err| {
                                HbciError::with_source(
                                    HbciErrorKind::Protocol,
                                    "failed to decode XML root namespace",
                                    err,
                                )
                            })?;
                        return Ok(Some(value.into_owned()));
                    }
                }
                return Ok(None);
            }
            Ok(Event::Decl(_)) | Ok(Event::Comment(_)) | Ok(Event::Text(_)) => {}
            Ok(Event::Eof) => return Ok(None),
            Ok(_) => {}
            Err(err) => {
                return Err(HbciError::with_source(
                    HbciErrorKind::Protocol,
                    "failed to parse XML document",
                    err,
                ));
            }
        }
    }
}
