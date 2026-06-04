mod model;

pub use model::{
    DefinitionKind, ProtocolSyntax, SyntaxChild, SyntaxChildKind, SyntaxDefinition, SyntaxValidSet,
    SyntaxValue,
};

use crate::error::{HbciError, HbciErrorKind, HbciResult};

pub const HBCI_DTD: &str = include_str!("../../resources/protocol/hbci.dtd");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSpec {
    pub version: &'static str,
    pub xml: &'static str,
}

impl ProtocolSpec {
    pub fn parse_syntax(&self) -> HbciResult<ProtocolSyntax> {
        ProtocolSyntax::parse(self)
    }

    pub fn deg_definition_count(&self) -> HbciResult<usize> {
        self.definition_count(b"DEGdef")
    }

    pub fn seg_definition_count(&self) -> HbciResult<usize> {
        self.definition_count(b"SEGdef")
    }

    pub fn sf_definition_count(&self) -> HbciResult<usize> {
        self.definition_count(b"SFdef")
    }

    pub fn msg_definition_count(&self) -> HbciResult<usize> {
        self.definition_count(b"MSGdef")
    }

    fn definition_count(&self, element_name: &[u8]) -> HbciResult<usize> {
        let mut reader = quick_xml::Reader::from_str(self.xml);
        reader.config_mut().trim_text(false);

        let mut count = 0;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Start(event))
                | Ok(quick_xml::events::Event::Empty(event)) => {
                    if event.name().as_ref() == element_name {
                        count += 1;
                    }
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => {}
                Err(err) => {
                    return Err(HbciError::with_source(
                        HbciErrorKind::Protocol,
                        format!("failed to parse HBCI {} protocol spec", self.version),
                        err,
                    ));
                }
            }
        }
        Ok(count)
    }
}

pub fn load_protocol_spec(version: &str) -> HbciResult<ProtocolSpec> {
    let (version, xml) = match version {
        "201" => ("201", include_str!("../../resources/protocol/hbci-201.xml")),
        "210" => ("210", include_str!("../../resources/protocol/hbci-210.xml")),
        "220" => ("220", include_str!("../../resources/protocol/hbci-220.xml")),
        "300" => ("300", include_str!("../../resources/protocol/hbci-300.xml")),
        "plus" => (
            "plus",
            include_str!("../../resources/protocol/hbci-plus.xml"),
        ),
        _ => {
            return Err(HbciError::new(
                HbciErrorKind::Unsupported,
                format!("unsupported HBCI version: {version}"),
            ));
        }
    };

    Ok(ProtocolSpec { version, xml })
}
