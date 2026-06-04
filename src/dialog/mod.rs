use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogContext {
    pub dialog_id: Option<String>,
    pub system_id: Option<String>,
    pub message_number: u32,
}
