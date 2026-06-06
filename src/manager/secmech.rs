use base64::Engine;

use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::tools::Properties;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatrixCode {
    mimetype: Option<String>,
    image: Vec<u8>,
}

impl MatrixCode {
    pub fn new(data: Option<&[u8]>) -> HbciResult<Self> {
        let data = data.ok_or_else(invalid_matrix_code)?;
        parse_image_payload(data).map(|payload| Self {
            mimetype: Some(payload.mimetype),
            image: payload.image,
        })
    }

    pub fn from_text(data: Option<&str>) -> HbciResult<Self> {
        Self::new(data.map(str::as_bytes))
    }

    pub fn try_parse(data: Option<&str>) -> Option<Self> {
        Self::from_text(data).ok()
    }

    pub fn image(&self) -> &[u8] {
        &self.image
    }

    pub fn mimetype(&self) -> Option<&str> {
        self.mimetype.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrCode {
    mimetype: Option<String>,
    message: Option<String>,
    image: Vec<u8>,
}

impl QrCode {
    pub fn new(hhd: Option<&str>, message: Option<&str>) -> HbciResult<Self> {
        let hhd_data = hhd.filter(|value| !value.is_empty()).map(str::as_bytes);

        if let Some(data) = hhd_data.filter(|data| data.len() > 100) {
            let payload = parse_image_payload(data)?;
            return Ok(Self {
                mimetype: Some(payload.mimetype),
                message: message.map(str::to_owned),
                image: payload.image,
            });
        }

        let message = message
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(invalid_qr_code)?;

        let compact = message
            .chars()
            .filter(|character| !matches!(character, '\n' | '\t' | '\r' | ' '))
            .collect::<String>();
        let challenge_start = compact.find("CHLGUC").ok_or_else(invalid_qr_code)?;
        let text_start = compact.find("CHLGTEXT").ok_or_else(invalid_qr_code)?;
        if text_start <= challenge_start {
            return Err(invalid_qr_code());
        }

        let encoded = compact
            .get(challenge_start..text_start)
            .and_then(|value| value.get(10..))
            .ok_or_else(invalid_qr_code)?;
        let image = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| {
                HbciError::with_source(HbciErrorKind::InvalidArgument, "invalid QR code", error)
            })?;
        let mimetype = if has_png_signature(&image) {
            Some("image/png".to_owned())
        } else {
            None
        };

        let text_start = message.find("CHLGTEXT").ok_or_else(invalid_qr_code)?;
        let text = message
            .get(text_start + 12..)
            .ok_or_else(invalid_qr_code)?
            .to_owned();

        Ok(Self {
            mimetype,
            message: Some(text),
            image,
        })
    }

    pub fn try_parse(hhd: Option<&str>, message: Option<&str>) -> Option<Self> {
        Self::new(hhd, message).ok()
    }

    pub fn image(&self) -> &[u8] {
        &self.image
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn mimetype(&self) -> Option<&str> {
        self.mimetype.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HhdVersion {
    Qr13,
    Qr14,
    Hhd14,
    Hhd13,
    Ms1,
    Hhd12,
    Decoupled,
}

impl HhdVersion {
    pub const DEFAULT: Self = Self::Hhd12;

    pub fn find(secmech: Option<&Properties>) -> Self {
        let Some(secmech) = secmech else {
            return Self::DEFAULT;
        };

        let name = property(secmech, "zkamethod_name").unwrap_or_default();
        if !name.is_empty() {
            for version in Self::ORDERED {
                if version.matches_name(name) {
                    return version;
                }
            }
        }

        let id = property(secmech, "id").unwrap_or_default();
        for version in Self::ORDERED {
            if version.matches_id(id) {
                return version;
            }
        }

        if let Some(zka_version) = property(secmech, "zkamethod_version")
            && !zka_version.is_empty()
        {
            for version in Self::ORDERED {
                if version.matches_zka_version(zka_version) {
                    return version;
                }
            }
        }

        if let Some(segversion) = property(secmech, "segversion")
            && let Ok(segversion) = segversion.parse::<i32>()
        {
            for version in Self::ORDERED {
                if version.matches_segment_version(segversion) {
                    return version;
                }
            }
        }

        Self::DEFAULT
    }

    pub fn hhd_type(self) -> HhdVersionType {
        match self {
            Self::Qr13 | Self::Qr14 => HhdVersionType::QrCode,
            Self::Ms1 => HhdVersionType::PhotoTan,
            Self::Decoupled => HhdVersionType::Decoupled,
            Self::Hhd14 | Self::Hhd13 | Self::Hhd12 => HhdVersionType::ChipTan,
        }
    }

    pub fn challenge_version(self) -> Option<&'static str> {
        match self {
            Self::Qr13 => Some("hhd13"),
            Self::Qr14 => Some("hhd14"),
            Self::Hhd14 => Some("hhd14"),
            Self::Hhd13 => Some("hhd13"),
            Self::Ms1 => Some("hhd14"),
            Self::Hhd12 => Some("hhd12"),
            Self::Decoupled => None,
        }
    }

    const ORDERED: [Self; 7] = [
        Self::Qr13,
        Self::Qr14,
        Self::Hhd14,
        Self::Hhd13,
        Self::Ms1,
        Self::Hhd12,
        Self::Decoupled,
    ];

    fn matches_name(self, name: &str) -> bool {
        match self {
            Self::Decoupled => name.starts_with("Decouple"),
            _ => false,
        }
    }

    fn matches_id(self, id: &str) -> bool {
        match self {
            Self::Qr13 => id.starts_with("HHD1.3.") && id.contains("QR"),
            Self::Qr14 => id.starts_with("Q1S"),
            Self::Hhd14 => id.starts_with("HHD1.4"),
            Self::Hhd13 => id.starts_with("HHD1.3"),
            Self::Ms1 => id.starts_with("MS1") || id.starts_with("photoTAN"),
            Self::Hhd12 | Self::Decoupled => false,
        }
    }

    fn matches_zka_version(self, version: &str) -> bool {
        match self {
            Self::Hhd14 => version.starts_with("1.4"),
            Self::Hhd13 => version.starts_with("1.3"),
            _ => false,
        }
    }

    fn matches_segment_version(self, segversion: i32) -> bool {
        match self {
            Self::Hhd14 => segversion == 5,
            Self::Hhd13 => segversion == 4,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HhdVersionType {
    ChipTan,
    PhotoTan,
    QrCode,
    Decoupled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ImagePayload {
    mimetype: String,
    image: Vec<u8>,
}

fn parse_image_payload(data: &[u8]) -> HbciResult<ImagePayload> {
    if data.len() < 100 {
        return Err(invalid_matrix_code());
    }

    let mut offset = 0;
    let Some(mimetype_len_bytes) = data.get(offset..offset + 2) else {
        return Err(invalid_matrix_code());
    };
    let mimetype_len = decode_decimal_len(mimetype_len_bytes)?;
    offset += 2;

    let Some(mimetype_bytes) = data.get(offset..offset + mimetype_len) else {
        return Err(invalid_matrix_code());
    };
    let mimetype = latin1_decode(mimetype_bytes);
    offset += mimetype_len;

    let Some(next_offset) = offset.checked_add(2) else {
        return Err(invalid_matrix_code());
    };
    offset = next_offset;
    let image = data.get(offset..).ok_or_else(invalid_matrix_code)?.to_vec();

    Ok(ImagePayload { mimetype, image })
}

fn decode_decimal_len(bytes: &[u8]) -> HbciResult<usize> {
    let value = bytes
        .iter()
        .map(|byte| i8::from_ne_bytes([*byte]).to_string())
        .collect::<String>();
    value.parse::<usize>().map_err(|error| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            "invalid image payload",
            error,
        )
    })
}

fn latin1_decode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn has_png_signature(bytes: &[u8]) -> bool {
    matches!(bytes, [0x89, b'P', b'N', b'G', ..])
}

fn property<'a>(properties: &'a Properties, key: &str) -> Option<&'a str> {
    properties.get(key).map(String::as_str)
}

fn invalid_matrix_code() -> HbciError {
    HbciError::new(HbciErrorKind::InvalidArgument, "invalid matrix code")
}

fn invalid_qr_code() -> HbciError {
    HbciError::new(HbciErrorKind::InvalidArgument, "invalid QR code")
}
