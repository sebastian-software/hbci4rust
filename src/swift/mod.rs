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

impl Mt940Document {
    pub fn parse(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}
