use ripemd::Ripemd160;
use sha1::{Digest, Sha1};

use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderHashMode {
    RipeMd160,
    Sha1,
}

impl OrderHashMode {
    pub fn from_code(code: &str) -> HbciResult<Self> {
        match code {
            "1" => Ok(Self::RipeMd160),
            "2" => Ok(Self::Sha1),
            value => Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("unknown orderhash mode {value}"),
            )),
        }
    }

    pub fn hash_segment(self, segment: &str) -> HbciResult<String> {
        Ok(latin1_string_from_bytes(&self.hash_segment_bytes(segment)?))
    }

    pub fn hash_segment_bin(self, segment: &str) -> HbciResult<String> {
        Ok(format!("B{}", self.hash_segment(segment)?))
    }

    pub fn hash_segment_bytes(self, segment: &str) -> HbciResult<Vec<u8>> {
        let data = latin1_bytes(segment)?;
        let digest = match self {
            Self::RipeMd160 => Ripemd160::digest(data).to_vec(),
            Self::Sha1 => Sha1::digest(data).to_vec(),
        };
        Ok(digest)
    }
}

fn latin1_bytes(value: &str) -> HbciResult<Vec<u8>> {
    let mut bytes = Vec::with_capacity(value.len());
    for character in value.chars() {
        let code = character as u32;
        if code > 0xff {
            return Err(HbciError::new(
                HbciErrorKind::Unsupported,
                format!("orderhash data is not ISO-8859-1 representable: U+{code:04X}"),
            ));
        }
        bytes.push(code as u8);
    }
    Ok(bytes)
}

fn latin1_string_from_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SALDO_SEGMENT: &str = "HKSAL:3:7+DE02123456780000000000+N'";

    #[test]
    fn maps_orderhash_modes_like_hbci4java() {
        assert_eq!(
            OrderHashMode::from_code("1").expect("mode 1"),
            OrderHashMode::RipeMd160
        );
        assert_eq!(
            OrderHashMode::from_code("2").expect("mode 2"),
            OrderHashMode::Sha1
        );
        assert!(OrderHashMode::from_code("0").is_err());
    }

    #[test]
    fn hashes_order_segments_like_hbci4java() {
        assert_eq!(
            hex(&OrderHashMode::Sha1
                .hash_segment_bytes(SALDO_SEGMENT)
                .expect("sha1 hash")),
            "1a1882a724fee3fb5f49a616b4fdefad1d6512f0"
        );
        assert_eq!(
            hex(&OrderHashMode::RipeMd160
                .hash_segment_bytes(SALDO_SEGMENT)
                .expect("ripemd160 hash")),
            "09e2b4cc96fb39674f06f0b21b3a2c80cb3404c9"
        );
    }

    #[test]
    fn creates_hktan_binary_orderhash_value() {
        let value = OrderHashMode::Sha1
            .hash_segment_bin(SALDO_SEGMENT)
            .expect("hktan binary value");

        assert!(value.starts_with('B'));
        assert_eq!(value.chars().count(), 21);
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
