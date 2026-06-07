use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UserSig {
    pin: String,
    tan: String,
}

impl UserSig {
    pub fn new(pin: impl Into<String>, tan: impl Into<String>) -> Self {
        Self {
            pin: pin.into(),
            tan: tan.into(),
        }
    }

    pub fn encode(pin: Option<&str>, tan: Option<&str>) -> HbciResult<Vec<u8>> {
        Self::new(pin.unwrap_or_default(), tan.unwrap_or_default()).to_bytes()
    }

    pub fn decode(sig: Option<&[u8]>) -> HbciResult<Self> {
        let Some(sig) = sig else {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "pin/tan missing or too short - sig length: 0",
            ));
        };
        if sig.is_empty() {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "pin/tan missing or too short - sig length: 0",
            ));
        }

        let pin_len = usize::from(sig[0]);
        if sig.len() < pin_len + 1 {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!(
                    "pin length invalid - sig length: {}, pin length: {pin_len}",
                    sig.len()
                ),
            ));
        }

        Ok(Self {
            pin: decode_latin1(&sig[1..pin_len + 1]),
            tan: decode_latin1(&sig[pin_len + 1..]),
        })
    }

    pub fn pin(&self) -> &str {
        &self.pin
    }

    pub fn tan(&self) -> &str {
        &self.tan
    }

    pub fn to_bytes(&self) -> HbciResult<Vec<u8>> {
        let pin = encode_latin1(&self.pin)?;
        if pin.len() > u8::MAX as usize {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("PIN is too long for UserSig length byte: {}", pin.len()),
            ));
        }

        let mut bytes = Vec::with_capacity(1 + pin.len() + self.tan.len());
        bytes.push(pin.len() as u8);
        bytes.extend(pin);
        bytes.extend(encode_latin1(&self.tan)?);
        Ok(bytes)
    }
}

fn encode_latin1(value: &str) -> HbciResult<Vec<u8>> {
    let mut bytes = Vec::with_capacity(value.len());
    for character in value.chars() {
        let code = character as u32;
        if code > 0xff {
            return Err(HbciError::new(
                HbciErrorKind::Unsupported,
                format!("UserSig character is not ISO-8859-1 representable: U+{code:04X}"),
            ));
        }
        bytes.push(code as u8);
    }
    Ok(bytes)
}

fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}
