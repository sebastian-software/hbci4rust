use std::collections::BTreeMap;

use base64::Engine;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};

use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::protocol::normalize_iso_date;
use crate::tools::Properties;

const FLICKER_LC_LENGTH_HHD14: usize = 3;
const FLICKER_LC_LENGTH_HHD13: usize = 2;
const FLICKER_LDE_LENGTH_DEFAULT: usize = 2;
const FLICKER_LDE_LENGTH_SPARDA: usize = 3;
const FLICKER_BIT_ENCODING: u8 = 6;
const FLICKER_BIT_CONTROLBYTE: u8 = 7;

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChallengeInfo {
    jobs: BTreeMap<String, ChallengeJob>,
}

impl ChallengeInfo {
    pub fn parse_xml(xml: &str) -> HbciResult<Self> {
        ChallengeInfoParser::default().parse(xml)
    }

    pub fn get_data(&self, code: &str) -> Option<&ChallengeJob> {
        self.jobs.get(code)
    }

    pub fn jobs(&self) -> &BTreeMap<String, ChallengeJob> {
        &self.jobs
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChallengeJob {
    versions: BTreeMap<String, ChallengeHhdVersion>,
}

impl ChallengeJob {
    pub fn version(&self, version: &str) -> Option<&ChallengeHhdVersion> {
        self.versions.get(version)
    }

    pub fn hhd_version(&self, version: HhdVersion) -> Option<&ChallengeHhdVersion> {
        version
            .challenge_version()
            .and_then(|version| self.version(version))
    }

    pub fn versions(&self) -> &BTreeMap<String, ChallengeHhdVersion> {
        &self.versions
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChallengeHhdVersion {
    klass: String,
    params: Vec<ChallengeParam>,
}

impl ChallengeHhdVersion {
    pub fn klass(&self) -> &str {
        &self.klass
    }

    pub fn params(&self) -> &[ChallengeParam] {
        &self.params
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChallengeParam {
    param_type: String,
    path: Option<String>,
    condition_name: Option<String>,
    condition_value: Option<String>,
}

impl ChallengeParam {
    pub fn param_type(&self) -> &str {
        &self.param_type
    }

    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    pub fn condition_name(&self) -> Option<&str> {
        self.condition_name.as_deref()
    }

    pub fn condition_value(&self) -> Option<&str> {
        self.condition_value.as_deref()
    }

    pub fn is_complied(&self, secmech: &Properties) -> bool {
        let Some(condition_name) = self.condition_name.as_deref() else {
            return true;
        };
        let expected = self.condition_value.as_deref().unwrap_or_default();
        property(secmech, condition_name).unwrap_or_default() == expected
    }

    pub fn format(&self, value: Option<&str>) -> HbciResult<Option<String>> {
        let Some(value) = value else {
            return Ok(None);
        };
        if value.trim().is_empty() {
            return Ok(None);
        }
        if self.param_type.trim().is_empty() {
            return Ok(Some(value.to_owned()));
        }

        let formatted = match self.param_type.as_str() {
            "Wrt" => format_challenge_wrt(value)?,
            "Date" => format_challenge_date(value)?,
            param_type => {
                return Err(HbciError::new(
                    HbciErrorKind::Unsupported,
                    format!("unsupported challenge parameter type: {param_type}"),
                ));
            }
        };
        Ok(Some(formatted))
    }
}

#[derive(Debug, Default)]
struct ChallengeInfoParser {
    info: ChallengeInfo,
    current_job_code: Option<String>,
    current_job: Option<ChallengeJob>,
    current_spec: Option<String>,
    current_hhd: Option<ChallengeHhdVersion>,
    current_param: Option<ChallengeParam>,
    text_capture: Option<ChallengeTextCapture>,
}

impl ChallengeInfoParser {
    fn parse(mut self, xml: &str) -> HbciResult<ChallengeInfo> {
        let mut reader = quick_xml::Reader::from_str(xml);
        reader.config_mut().trim_text(false);

        loop {
            match reader.read_event() {
                Ok(Event::Start(event)) => self.handle_start(&reader, event)?,
                Ok(Event::Empty(event)) => self.handle_empty(&reader, event)?,
                Ok(Event::Text(event)) => {
                    if let Some(capture) = &mut self.text_capture {
                        let text = event.decode().map_err(|err| {
                            HbciError::with_source(
                                HbciErrorKind::Protocol,
                                "failed to decode challenge info text",
                                err,
                            )
                        })?;
                        capture.push_str(&text);
                    }
                }
                Ok(Event::End(event)) => self.handle_end(event.name().as_ref())?,
                Ok(Event::Eof) => break,
                Ok(_) => {}
                Err(err) => {
                    return Err(HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to parse challenge info XML",
                        err,
                    ));
                }
            }
        }

        Ok(self.info)
    }

    fn handle_start(
        &mut self,
        reader: &quick_xml::Reader<&[u8]>,
        event: BytesStart<'_>,
    ) -> HbciResult<()> {
        match event.name().as_ref() {
            b"job" => {
                self.current_job_code = Some(challenge_required_attr(reader, &event, b"code")?);
                self.current_job = Some(ChallengeJob::default());
            }
            b"challengeinfo" => {
                self.current_spec = Some(challenge_required_attr(reader, &event, b"spec")?);
                self.current_hhd = Some(ChallengeHhdVersion::default());
            }
            b"klass" => {
                self.text_capture = Some(ChallengeTextCapture::Klass(String::new()));
            }
            b"param" => {
                self.current_param = Some(parse_challenge_param(reader, &event)?);
                self.text_capture = Some(ChallengeTextCapture::Param(String::new()));
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_empty(
        &mut self,
        reader: &quick_xml::Reader<&[u8]>,
        event: BytesStart<'_>,
    ) -> HbciResult<()> {
        if event.name().as_ref() == b"param" {
            let param = parse_challenge_param(reader, &event)?;
            self.current_hhd
                .as_mut()
                .ok_or_else(challenge_info_stack_underflow)?
                .params
                .push(param);
        }
        Ok(())
    }

    fn handle_end(&mut self, name: &[u8]) -> HbciResult<()> {
        match name {
            b"klass" => {
                let Some(ChallengeTextCapture::Klass(klass)) = self.text_capture.take() else {
                    return Ok(());
                };
                self.current_hhd
                    .as_mut()
                    .ok_or_else(challenge_info_stack_underflow)?
                    .klass = klass;
            }
            b"param" => {
                let mut param = self
                    .current_param
                    .take()
                    .ok_or_else(challenge_info_stack_underflow)?;
                if let Some(ChallengeTextCapture::Param(path)) = self.text_capture.take()
                    && !path.is_empty()
                {
                    param.path = Some(path);
                }
                self.current_hhd
                    .as_mut()
                    .ok_or_else(challenge_info_stack_underflow)?
                    .params
                    .push(param);
            }
            b"challengeinfo" => {
                let spec = self
                    .current_spec
                    .take()
                    .ok_or_else(challenge_info_stack_underflow)?;
                let hhd = self
                    .current_hhd
                    .take()
                    .ok_or_else(challenge_info_stack_underflow)?;
                self.current_job
                    .as_mut()
                    .ok_or_else(challenge_info_stack_underflow)?
                    .versions
                    .insert(spec, hhd);
            }
            b"job" => {
                let code = self
                    .current_job_code
                    .take()
                    .ok_or_else(challenge_info_stack_underflow)?;
                let job = self
                    .current_job
                    .take()
                    .ok_or_else(challenge_info_stack_underflow)?;
                self.info.jobs.insert(code, job);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ChallengeTextCapture {
    Klass(String),
    Param(String),
}

impl ChallengeTextCapture {
    fn push_str(&mut self, text: &str) {
        match self {
            Self::Klass(value) | Self::Param(value) => value.push_str(text),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlickerCode {
    pub version: Option<FlickerCodeVersion>,
    pub lc: u32,
    pub start_code: FlickerStartCode,
    pub de1: FlickerDataElement,
    pub de2: FlickerDataElement,
    pub de3: FlickerDataElement,
    pub rest: Option<String>,
}

impl FlickerCode {
    pub fn new(code: &str) -> HbciResult<Self> {
        Self::with_hhd(None, code)
    }

    pub fn with_hhd(hhd: Option<HhdVersion>, code: &str) -> HbciResult<Self> {
        if let Some(hhd) = hhd
            && let Some(version) = FlickerCodeVersion::from_hhd_version(hhd)
            && let Ok(code) = Self::parse(code, version, FLICKER_LDE_LENGTH_DEFAULT)
        {
            return Ok(code);
        }

        Self::parse(code, FlickerCodeVersion::Hhd14, FLICKER_LDE_LENGTH_DEFAULT)
            .or_else(|_| Self::parse(code, FlickerCodeVersion::Hhd14, FLICKER_LDE_LENGTH_SPARDA))
            .or_else(|_| Self::parse(code, FlickerCodeVersion::Hhd13, FLICKER_LDE_LENGTH_DEFAULT))
    }

    pub fn try_parse(
        hhd: Option<HhdVersion>,
        challenge: Option<&str>,
        hhduc: Option<&str>,
    ) -> Option<Self> {
        if let Some(hhduc) = hhduc.filter(|value| !value.trim().is_empty())
            && let Ok(code) = Self::with_hhd(hhd, hhduc)
            && code.render().is_ok()
        {
            return Some(code);
        }

        if let Some(challenge) = challenge.filter(|value| !value.trim().is_empty())
            && let Ok(code) = Self::with_hhd(hhd, challenge)
            && code.render().is_ok()
        {
            return Some(code);
        }

        None
    }

    pub fn render(&self) -> HbciResult<String> {
        let payload = self.create_payload()?;
        let luhn = self.create_luhn_checksum()?;
        let xor = create_xor_checksum(&payload)?;
        Ok(format!("{payload}{luhn}{xor}"))
    }

    fn parse(code: &str, version: FlickerCodeVersion, lde_len: usize) -> HbciResult<Self> {
        let code = clean_flicker_code(code);
        let lc_len = match version {
            FlickerCodeVersion::Hhd14 => FLICKER_LC_LENGTH_HHD14,
            FlickerCodeVersion::Hhd13 => FLICKER_LC_LENGTH_HHD13,
        };

        let (lc, rest) = take_prefix(&code, lc_len)?;
        let lc = parse_decimal(lc)?;
        let (start_code, parsed_version, rest) = FlickerStartCode::parse(rest)?;
        let (de1, rest) = FlickerDataElement::parse(rest, lde_len)?;
        let (de2, rest) = FlickerDataElement::parse(rest, lde_len)?;
        let (de3, rest) = FlickerDataElement::parse(rest, lde_len)?;

        Ok(Self {
            version: Some(parsed_version),
            lc,
            start_code,
            de1,
            de2,
            de3,
            rest: (!rest.is_empty()).then(|| rest.to_owned()),
        })
    }

    fn create_payload(&self) -> HbciResult<String> {
        let mut payload = String::new();
        payload.push_str(&self.start_code.render_length(self.version_or_error()?)?);
        for control_byte in &self.start_code.control_bytes {
            payload.push_str(&to_hex(*control_byte, 2));
        }
        payload.push_str(&self.start_code.element.render_data()?);

        for de in [&self.de1, &self.de2, &self.de3] {
            payload.push_str(&de.render_length(self.version_or_error()?)?);
            payload.push_str(&de.render_data()?);
        }

        let byte_len = (payload.len() + 2) / 2;
        Ok(format!("{}{}", to_hex(byte_len as u32, 2), payload))
    }

    fn create_luhn_checksum(&self) -> HbciResult<String> {
        let mut payload = String::new();
        for control_byte in &self.start_code.control_bytes {
            payload.push_str(&to_hex(*control_byte, 2));
        }
        payload.push_str(&self.start_code.element.render_data()?);
        for de in [&self.de1, &self.de2, &self.de3] {
            if de.data.is_some() {
                payload.push_str(&de.render_data()?);
            }
        }

        if !payload.len().is_multiple_of(2) {
            return Err(invalid_flicker_code());
        }

        let mut luhn_sum = 0;
        let bytes = payload.as_bytes();
        for index in (0..bytes.len()).step_by(2) {
            luhn_sum += hex_digit(bytes[index])?;
            luhn_sum += digit_sum(2 * hex_digit(bytes[index + 1])?);
        }

        let modulo = luhn_sum % 10;
        if modulo == 0 {
            return Ok("0".to_owned());
        }

        Ok(to_hex(10 - modulo, 1))
    }

    fn version_or_error(&self) -> HbciResult<FlickerCodeVersion> {
        self.version.ok_or_else(invalid_flicker_code)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlickerCodeVersion {
    Hhd14,
    Hhd13,
}

impl FlickerCodeVersion {
    fn from_hhd_version(version: HhdVersion) -> Option<Self> {
        match version {
            HhdVersion::Qr14 | HhdVersion::Hhd14 | HhdVersion::Ms1 => Some(Self::Hhd14),
            HhdVersion::Qr13 | HhdVersion::Hhd13 | HhdVersion::Hhd12 => Some(Self::Hhd13),
            HhdVersion::Decoupled => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlickerEncoding {
    Asc,
    Bcd,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlickerDataElement {
    pub length: u32,
    pub lde: u32,
    pub lde_len: usize,
    pub encoding: Option<FlickerEncoding>,
    pub data: Option<String>,
}

impl FlickerDataElement {
    fn parse(input: &str, lde_len: usize) -> HbciResult<(Self, &str)> {
        if input.is_empty() {
            return Ok((Self::default(), input));
        }

        let (lde, rest) = take_prefix(input, lde_len)?;
        let lde = parse_decimal(lde)?;
        let length = get_bit_sum(lde, 5);
        let (data, rest) = take_prefix(rest, length as usize)?;

        Ok((
            Self {
                length,
                lde,
                lde_len,
                encoding: None,
                data: Some(data.to_owned()),
            },
            rest,
        ))
    }

    fn render_length(&self, version: FlickerCodeVersion) -> HbciResult<String> {
        if self.data.is_none() {
            return Ok(String::new());
        }

        let encoding = self.resolved_encoding();
        let mut byte_len = self.render_data()?.len() / 2;
        if encoding == FlickerEncoding::Bcd {
            return Ok(to_hex(byte_len as u32, 2));
        }

        if version == FlickerCodeVersion::Hhd14 {
            byte_len += 1 << FLICKER_BIT_ENCODING;
            return Ok(to_hex(byte_len as u32, 2));
        }

        Ok(format!("1{}", to_hex(byte_len as u32, 1)))
    }

    fn render_data(&self) -> HbciResult<String> {
        let Some(data) = &self.data else {
            return Ok(String::new());
        };

        if self.resolved_encoding() == FlickerEncoding::Asc {
            return Ok(to_hex_string(data));
        }

        let mut rendered = data.clone();
        if rendered.len() % 2 == 1 {
            rendered.push('F');
        }
        Ok(rendered)
    }

    fn resolved_encoding(&self) -> FlickerEncoding {
        if let Some(encoding) = self.encoding {
            return encoding;
        }

        if self
            .data
            .as_deref()
            .is_some_and(|data| !data.is_empty() && data.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return FlickerEncoding::Bcd;
        }

        FlickerEncoding::Asc
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlickerStartCode {
    pub element: FlickerDataElement,
    pub control_bytes: Vec<u32>,
}

impl FlickerStartCode {
    fn parse(input: &str) -> HbciResult<(Self, FlickerCodeVersion, &str)> {
        let (lde, rest) = take_prefix(input, 2)?;
        let lde = parse_hex(lde)?;
        let length = get_bit_sum(lde, 5);
        let mut rest = rest;
        let mut version = FlickerCodeVersion::Hhd13;
        let mut control_bytes = Vec::new();

        if is_bit_set(lde, FLICKER_BIT_CONTROLBYTE) {
            version = FlickerCodeVersion::Hhd14;
            for _ in 0..10 {
                let (control_byte, next) = take_prefix(rest, 2)?;
                let control_byte = parse_hex(control_byte)?;
                control_bytes.push(control_byte);
                rest = next;

                if !is_bit_set(control_byte, FLICKER_BIT_CONTROLBYTE) {
                    break;
                }
            }
        }

        let (data, rest) = take_prefix(rest, length as usize)?;
        let element = FlickerDataElement {
            length,
            lde,
            lde_len: 0,
            encoding: None,
            data: Some(data.to_owned()),
        };

        Ok((
            Self {
                element,
                control_bytes,
            },
            version,
            rest,
        ))
    }

    fn render_length(&self, version: FlickerCodeVersion) -> HbciResult<String> {
        let mut rendered = self.element.render_length(version)?;
        if version == FlickerCodeVersion::Hhd13 || self.control_bytes.is_empty() {
            return Ok(rendered);
        }

        let mut len = parse_hex(&rendered)?;
        len += 1 << FLICKER_BIT_CONTROLBYTE;
        rendered = to_hex(len, 2);
        Ok(rendered)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlickerRenderer {
    bit_array: Vec<[bool; 5]>,
}

impl FlickerRenderer {
    pub const FREQUENCY_DEFAULT: u32 = 10;
    pub const FREQUENCY_MIN: u32 = 2;
    pub const FREQUENCY_MAX: u32 = 40;

    pub fn new(code: &str) -> HbciResult<Self> {
        let code = format!("0FFF{code}");
        if code.len() % 2 != 0 {
            return Err(invalid_flicker_code());
        }

        let mut bit_array = Vec::new();
        let chars = code.as_bytes();
        for index in (0..chars.len()).step_by(2) {
            bit_array.push(flicker_bits(chars[index + 1])?);
            bit_array.push(flicker_bits(chars[index])?);
        }

        Ok(Self { bit_array })
    }

    pub fn frames_for_iterations(&self, iterations: usize) -> Vec<[bool; 5]> {
        let mut frames = Vec::with_capacity(self.bit_array.len() * iterations * 2);
        for _ in 0..iterations {
            for bits in &self.bit_array {
                let mut high_clock = *bits;
                high_clock[0] = true;
                frames.push(high_clock);

                let mut low_clock = *bits;
                low_clock[0] = false;
                frames.push(low_clock);
            }
        }
        frames
    }
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

fn parse_challenge_param(
    reader: &quick_xml::Reader<&[u8]>,
    event: &BytesStart<'_>,
) -> HbciResult<ChallengeParam> {
    Ok(ChallengeParam {
        param_type: challenge_attr(reader, event, b"type")?.unwrap_or_default(),
        path: None,
        condition_name: challenge_attr(reader, event, b"condition-name")?,
        condition_value: challenge_attr(reader, event, b"condition-value")?,
    })
}

fn challenge_required_attr(
    reader: &quick_xml::Reader<&[u8]>,
    event: &BytesStart<'_>,
    name: &[u8],
) -> HbciResult<String> {
    challenge_attr(reader, event, name)?.ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::Protocol,
            format!(
                "challenge info element {} is missing required {} attribute",
                String::from_utf8_lossy(event.name().as_ref()),
                String::from_utf8_lossy(name)
            ),
        )
    })
}

fn challenge_attr(
    reader: &quick_xml::Reader<&[u8]>,
    event: &BytesStart<'_>,
    name: &[u8],
) -> HbciResult<Option<String>> {
    for attr in event.attributes().with_checks(false) {
        let attr = attr.map_err(|err| {
            HbciError::with_source(
                HbciErrorKind::Protocol,
                "failed to parse challenge info XML attribute",
                err,
            )
        })?;
        if attr.key.as_ref() == name {
            let value = attr
                .decoded_and_normalized_value(XmlVersion::Explicit1_0, reader.decoder())
                .map_err(|err| {
                    HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to decode challenge info XML attribute",
                        err,
                    )
                })?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

fn format_challenge_wrt(value: &str) -> HbciResult<String> {
    let value = value.trim().replace(',', ".");
    let parsed = value.parse::<f64>().map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            format!("invalid Wrt challenge parameter value: {value}"),
            err,
        )
    })?;

    if !parsed.is_finite() {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("invalid Wrt challenge parameter value: {value}"),
        ));
    }

    let rounded = (parsed * 100.0).round() / 100.0;
    let mut rendered = format!("{rounded:.2}");
    while rendered.ends_with('0') {
        rendered.pop();
    }
    Ok(rendered.replace('.', ","))
}

fn format_challenge_date(value: &str) -> HbciResult<String> {
    let date = normalize_iso_date(value)?;
    Ok(date.replace('-', ""))
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

fn clean_flicker_code(code: &str) -> String {
    let mut code = code.replace(' ', "");
    code = code.trim().to_owned();

    let challenge_start = code.find("CHLGUC");
    let text_start = code.find("CHLGTEXT");
    let (Some(challenge_start), Some(text_start)) = (challenge_start, text_start) else {
        return code;
    };
    if text_start <= challenge_start {
        return code;
    }

    let code = &code[challenge_start..text_start];
    let Some(code) = code.get(10..) else {
        return String::new();
    };
    format!("0{code}")
}

fn take_prefix(input: &str, len: usize) -> HbciResult<(&str, &str)> {
    if input.len() < len {
        return Err(invalid_flicker_code());
    }
    Ok((&input[..len], &input[len..]))
}

fn parse_decimal(value: &str) -> HbciResult<u32> {
    value.parse::<u32>().map_err(|error| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            "invalid flicker code",
            error,
        )
    })
}

fn parse_hex(value: &str) -> HbciResult<u32> {
    u32::from_str_radix(value, 16).map_err(|error| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            "invalid flicker code",
            error,
        )
    })
}

fn to_hex(value: u32, len: usize) -> String {
    let mut rendered = format!("{value:X}");
    while rendered.len() < len {
        rendered.insert(0, '0');
    }
    rendered
}

fn to_hex_string(value: &str) -> String {
    value
        .chars()
        .map(|character| to_hex(character as u32, 2))
        .collect()
}

fn create_xor_checksum(payload: &str) -> HbciResult<String> {
    let mut xor_sum = 0;
    for byte in payload.bytes() {
        xor_sum ^= hex_digit(byte)?;
    }
    Ok(to_hex(xor_sum, 1))
}

fn hex_digit(byte: u8) -> HbciResult<u32> {
    match byte {
        b'0'..=b'9' => Ok(u32::from(byte - b'0')),
        b'A'..=b'F' => Ok(u32::from(byte - b'A' + 10)),
        b'a'..=b'f' => Ok(u32::from(byte - b'a' + 10)),
        _ => Err(invalid_flicker_code()),
    }
}

fn digit_sum(mut value: u32) -> u32 {
    let mut sum = 0;
    while value != 0 {
        sum += value % 10;
        value /= 10;
    }
    sum
}

fn get_bit_sum(value: u32, bits: u8) -> u32 {
    (0..=bits).map(|bit| value & (1 << bit)).sum()
}

fn is_bit_set(value: u32, bit: u8) -> bool {
    value & (1 << bit) != 0
}

fn flicker_bits(byte: u8) -> HbciResult<[bool; 5]> {
    let bits = match byte {
        b'0' => [false, false, false, false, false],
        b'1' => [false, true, false, false, false],
        b'2' => [false, false, true, false, false],
        b'3' => [false, true, true, false, false],
        b'4' => [false, false, false, true, false],
        b'5' => [false, true, false, true, false],
        b'6' => [false, false, true, true, false],
        b'7' => [false, true, true, true, false],
        b'8' => [false, false, false, false, true],
        b'9' => [false, true, false, false, true],
        b'A' | b'a' => [false, false, true, false, true],
        b'B' | b'b' => [false, true, true, false, true],
        b'C' | b'c' => [false, false, false, true, true],
        b'D' | b'd' => [false, true, false, true, true],
        b'E' | b'e' => [false, false, true, true, true],
        b'F' | b'f' => [false, true, true, true, true],
        _ => return Err(invalid_flicker_code()),
    };
    Ok(bits)
}

fn invalid_matrix_code() -> HbciError {
    HbciError::new(HbciErrorKind::InvalidArgument, "invalid matrix code")
}

fn invalid_qr_code() -> HbciError {
    HbciError::new(HbciErrorKind::InvalidArgument, "invalid QR code")
}

fn invalid_flicker_code() -> HbciError {
    HbciError::new(HbciErrorKind::InvalidArgument, "invalid flicker code")
}

fn challenge_info_stack_underflow() -> HbciError {
    HbciError::new(
        HbciErrorKind::Protocol,
        "challenge info XML parser stack underflow",
    )
}
