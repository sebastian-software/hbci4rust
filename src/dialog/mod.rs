use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogContext {
    pub dialog_id: Option<String>,
    pub system_id: Option<String>,
    pub message_number: u32,
}

impl DialogContext {
    pub fn from_dialog_id(dialog_id: impl Into<String>) -> Self {
        Self {
            dialog_id: Some(dialog_id.into()),
            system_id: None,
            message_number: 2,
        }
    }

    pub fn current_dialog_id(&self) -> &str {
        self.dialog_id.as_deref().unwrap_or("0")
    }

    pub fn open_dialog_id(&self) -> Option<&str> {
        self.dialog_id.as_deref()
    }

    pub fn is_open(&self) -> bool {
        self.dialog_id.is_some()
    }

    pub fn current_message_number(&self) -> u32 {
        self.message_number.max(1)
    }

    pub fn advance_message_number(&mut self) {
        self.message_number = self.current_message_number().saturating_add(1);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnownReturncode {
    W3040,
    W3072,
    W3076,
    W3091,
    W3920,
    W3945,
    W3956,
    E9340,
    E9930,
    E9931,
    E9942,
    E9391,
}

impl KnownReturncode {
    pub const LIST_AUTH_FAIL: [Self; 4] = [Self::E9340, Self::E9930, Self::E9931, Self::E9942];

    pub const fn code(self) -> &'static str {
        match self {
            Self::W3040 => "3040",
            Self::W3072 => "3072",
            Self::W3076 => "3076",
            Self::W3091 => "3091",
            Self::W3920 => "3920",
            Self::W3945 => "3945",
            Self::W3956 => "3956",
            Self::E9340 => "9340",
            Self::E9930 => "9930",
            Self::E9931 => "9931",
            Self::E9942 => "9942",
            Self::E9391 => "9391",
        }
    }

    pub fn is(self, code: &str) -> bool {
        self.code() == code
    }

    pub fn contains(code: &str, codes: &[Self]) -> bool {
        Self::find(code, codes).is_some()
    }

    pub fn find(code: &str, codes: &[Self]) -> Option<Self> {
        if code.is_empty() || codes.is_empty() {
            return None;
        }

        codes.iter().copied().find(|known| known.is(code))
    }
}

impl Default for DialogContext {
    fn default() -> Self {
        Self {
            dialog_id: None,
            system_id: None,
            message_number: 1,
        }
    }
}
