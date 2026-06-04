use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub type HbciResult<T> = Result<T, HbciError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HbciErrorKind {
    Callback,
    Config,
    InvalidArgument,
    Network,
    Protocol,
    Storage,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HbciError {
    kind: HbciErrorKind,
    message: String,
    source: Option<String>,
}

impl HbciError {
    pub fn new(kind: HbciErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(
        kind: HbciErrorKind,
        message: impl Into<String>,
        source: impl Display,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(source.to_string()),
        }
    }

    pub fn kind(&self) -> HbciErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_message(&self) -> Option<&str> {
        self.source.as_deref()
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(HbciErrorKind::Unsupported, message)
    }
}

impl Display for HbciError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(f, "{}: {}", self.message, source),
            None => f.write_str(&self.message),
        }
    }
}

impl Error for HbciError {}
