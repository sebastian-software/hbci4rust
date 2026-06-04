use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSpec {
    pub version: &'static str,
    pub xml: &'static str,
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
