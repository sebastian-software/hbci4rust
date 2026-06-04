#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mt940Document {
    pub raw: String,
}

impl Mt940Document {
    pub fn parse(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }
}
