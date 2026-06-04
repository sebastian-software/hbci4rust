use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinTanPassport {
    data: PinTanPassportData,
}

impl PinTanPassport {
    pub fn new(data: PinTanPassportData) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &PinTanPassportData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut PinTanPassportData {
        &mut self.data
    }

    pub fn host(&self) -> Option<&str> {
        self.data.host.as_deref()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinTanPassportData {
    pub country: String,
    pub blz: String,
    pub host: Option<String>,
    pub user_id: String,
    pub customer_id: Option<String>,
    pub filter: Option<String>,
    pub tan_method: Option<String>,
    pub tan_media: Option<String>,
}
