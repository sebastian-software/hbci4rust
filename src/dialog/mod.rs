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

    pub fn current_message_number(&self) -> u32 {
        self.message_number.max(1)
    }

    pub fn advance_message_number(&mut self) {
        self.message_number = self.current_message_number().saturating_add(1);
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
