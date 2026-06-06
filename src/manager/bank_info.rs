use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankInfo {
    blz: Option<String>,
    bic: Option<String>,
    checksum_method: Option<String>,
    location: Option<String>,
    name: Option<String>,
    pin_tan_address: Option<String>,
    pin_tan_version: Option<HbciVersion>,
    rdh_address: Option<String>,
    rdh_version: Option<HbciVersion>,
}

impl BankInfo {
    pub fn parse_value(text: &str) -> Self {
        let mut info = Self::empty();
        if text.is_empty() {
            return info;
        }

        let columns = java_property_columns(text);
        info.name = column_value(&columns, 0);
        info.location = column_value(&columns, 1);
        info.bic = column_value(&columns, 2);
        info.checksum_method = column_value(&columns, 3);
        info.rdh_address = column_value(&columns, 4);
        info.pin_tan_address = column_value(&columns, 5);
        info.rdh_version = column_value(&columns, 6).and_then(|value| HbciVersion::by_id(&value));
        info.pin_tan_version =
            column_value(&columns, 7).and_then(|value| HbciVersion::by_id(&value));

        info
    }

    pub fn parse_property(blz: impl Into<String>, value: &str) -> Self {
        let mut info = Self::parse_value(value);
        info.blz = Some(blz.into());
        info
    }

    pub fn empty() -> Self {
        Self {
            blz: None,
            bic: None,
            checksum_method: None,
            location: None,
            name: None,
            pin_tan_address: None,
            pin_tan_version: None,
            rdh_address: None,
            rdh_version: None,
        }
    }

    pub fn blz(&self) -> Option<&str> {
        self.blz.as_deref()
    }

    pub fn bic(&self) -> Option<&str> {
        self.bic.as_deref()
    }

    pub fn checksum_method(&self) -> Option<&str> {
        self.checksum_method.as_deref()
    }

    pub fn location(&self) -> Option<&str> {
        self.location.as_deref()
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn pin_tan_address(&self) -> Option<&str> {
        self.pin_tan_address.as_deref()
    }

    pub fn pin_tan_version(&self) -> Option<HbciVersion> {
        self.pin_tan_version
    }

    pub fn rdh_address(&self) -> Option<&str> {
        self.rdh_address.as_deref()
    }

    pub fn rdh_version(&self) -> Option<HbciVersion> {
        self.rdh_version
    }
}

impl Display for BankInfo {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}",
            self.blz.as_deref().unwrap_or("null"),
            self.name.as_deref().unwrap_or("null")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbciVersion {
    Hbci201,
    Hbci210,
    Hbci220,
    HbciPlus,
    Hbci300,
    Hbci400,
}

impl HbciVersion {
    pub fn by_id(id: &str) -> Option<Self> {
        match id {
            "201" => Some(Self::Hbci201),
            "210" => Some(Self::Hbci210),
            "220" => Some(Self::Hbci220),
            "plus" => Some(Self::HbciPlus),
            "300" => Some(Self::Hbci300),
            "400" => Some(Self::Hbci400),
            _ => None,
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Hbci201 => "201",
            Self::Hbci210 => "210",
            Self::Hbci220 => "220",
            Self::HbciPlus => "plus",
            Self::Hbci300 => "300",
            Self::Hbci400 => "400",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Hbci201 => "HBCI 2.01",
            Self::Hbci210 => "HBCI 2.10",
            Self::Hbci220 => "HBCI 2.2",
            Self::HbciPlus => "HBCI 2.2 (HBCI+)",
            Self::Hbci300 => "FinTS 3.0",
            Self::Hbci400 => "FinTS 4.0",
        }
    }
}

impl Display for HbciVersion {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.id(), self.name())
    }
}

fn java_property_columns(text: &str) -> Vec<&str> {
    let mut columns = text.split('|').collect::<Vec<_>>();
    while columns.last().is_some_and(|value| value.is_empty()) {
        columns.pop();
    }
    columns
}

fn column_value(columns: &[&str], index: usize) -> Option<String> {
    columns.get(index).map(|value| (*value).to_owned())
}
