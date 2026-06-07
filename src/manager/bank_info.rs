use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::sync::OnceLock;

const BUNDLED_BLZ_PROPERTIES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/bank_info/blz.properties"
));

static BUNDLED_BANK_INFO_REGISTRY: OnceLock<BankInfoRegistry> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BankInfoRegistry {
    banks: BTreeMap<String, BankInfo>,
}

impl BankInfoRegistry {
    pub fn bundled() -> &'static Self {
        BUNDLED_BANK_INFO_REGISTRY.get_or_init(|| Self::parse_properties(BUNDLED_BLZ_PROPERTIES))
    }

    pub fn parse_properties(text: &str) -> Self {
        let mut registry = Self::default();

        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            let line = line.trim_start();
            if line.is_empty() || line.starts_with('#') || line.starts_with('!') {
                continue;
            }

            let (blz, value) = split_property_line(line).unwrap_or((line, ""));
            let blz = blz.trim_end();
            if blz.is_empty() {
                continue;
            }

            registry.banks.insert(
                blz.to_owned(),
                BankInfo::parse_property(blz, value.trim_start()),
            );
        }

        registry
    }

    pub fn get_bank_info(&self, blz: &str) -> Option<&BankInfo> {
        self.banks.get(blz)
    }

    pub fn name_for_blz(&self, blz: &str) -> &str {
        self.get_bank_info(blz)
            .and_then(BankInfo::name)
            .unwrap_or("")
    }

    pub fn search_bank_info(&self, query: &str) -> Vec<&BankInfo> {
        let query = query.trim();
        if query.chars().count() < 3 {
            return Vec::new();
        }

        let query = query.to_lowercase();
        self.banks
            .values()
            .filter(|info| bank_info_matches_query(info, &query))
            .collect()
    }

    pub fn banks(&self) -> impl Iterator<Item = &BankInfo> {
        self.banks.values()
    }

    pub fn pin_tan_banks(&self) -> impl Iterator<Item = &BankInfo> {
        self.banks().filter(|info| info.supports_pin_tan())
    }

    pub fn search_pin_tan_banks(&self, query: &str) -> Vec<&BankInfo> {
        self.search_bank_info(query)
            .into_iter()
            .filter(|info| info.supports_pin_tan())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.banks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.banks.is_empty()
    }
}

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

    pub fn supports_pin_tan(&self) -> bool {
        has_text(self.pin_tan_address()) || self.pin_tan_version.is_some()
    }

    pub fn rdh_address(&self) -> Option<&str> {
        self.rdh_address.as_deref()
    }

    pub fn rdh_version(&self) -> Option<HbciVersion> {
        self.rdh_version
    }

    pub fn supports_rdh(&self) -> bool {
        has_text(self.rdh_address()) || self.rdh_version.is_some()
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

fn bank_info_matches_query(info: &BankInfo, query: &str) -> bool {
    info.blz().is_some_and(|blz| blz.starts_with(query))
        || info
            .bic()
            .is_some_and(|bic| bic.to_lowercase().starts_with(query))
        || info
            .name()
            .is_some_and(|name| name.to_lowercase().contains(query))
        || info
            .location()
            .is_some_and(|location| location.to_lowercase().contains(query))
}

fn split_property_line(line: &str) -> Option<(&str, &str)> {
    let separator = match (line.find('='), line.find(':')) {
        (Some(equals), Some(colon)) => Some(equals.min(colon)),
        (Some(equals), None) => Some(equals),
        (None, Some(colon)) => Some(colon),
        (None, None) => None,
    }?;

    Some((&line[..separator], &line[separator + 1..]))
}

fn has_text(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}
