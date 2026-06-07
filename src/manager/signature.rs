use std::time::{SystemTime, UNIX_EPOCH};

use rand_core::{OsRng, RngCore};

use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::passport::{ONESTEP_TAN_METHOD_ID, PinTanPassport, UserSig};
use crate::protocol::{HbciMessage, SyntaxElement};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinTanSignatureContext {
    pub seccheckref: String,
    pub secref: String,
    pub timestamp_date: String,
    pub timestamp_time: String,
}

impl PinTanSignatureContext {
    pub fn new(
        seccheckref: impl Into<String>,
        secref: impl Into<String>,
        timestamp_date: impl Into<String>,
        timestamp_time: impl Into<String>,
    ) -> Self {
        Self {
            seccheckref: seccheckref.into(),
            secref: secref.into(),
            timestamp_date: timestamp_date.into(),
            timestamp_time: timestamp_time.into(),
        }
    }

    pub fn generate() -> HbciResult<Self> {
        Self::from_system_time(random_seccheckref(), SystemTime::now())
    }

    pub fn from_system_time(
        seccheckref: impl Into<String>,
        system_time: SystemTime,
    ) -> HbciResult<Self> {
        let (timestamp_date, timestamp_time) = fints_timestamp_from_system_time(system_time)?;
        Ok(Self::new(seccheckref, "1", timestamp_date, timestamp_time))
    }

    pub fn sig_head_from_passport(&self, passport: &PinTanPassport) -> HbciResult<PinTanSigHead> {
        PinTanSigHead::from_passport(
            passport,
            &self.seccheckref,
            &self.secref,
            &self.timestamp_date,
            &self.timestamp_time,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinTanSigHead {
    pub profile_method: String,
    pub profile_version: String,
    pub secfunc: String,
    pub seccheckref: String,
    pub role: String,
    pub sec_idn_func: String,
    pub sys_id: String,
    pub secref: String,
    pub timestamp_date: String,
    pub timestamp_time: String,
    pub hash_alg: String,
    pub sig_alg: String,
    pub sig_mode: String,
    pub key_country: String,
    pub key_blz: String,
    pub key_user_id: String,
    pub key_num: String,
    pub key_version: String,
}

impl PinTanSigHead {
    pub fn from_passport(
        passport: &PinTanPassport,
        seccheckref: impl Into<String>,
        secref: impl Into<String>,
        timestamp_date: impl Into<String>,
        timestamp_time: impl Into<String>,
    ) -> HbciResult<Self> {
        let data = passport.data();
        let key_country = if data.country.is_empty() {
            "DE".to_owned()
        } else {
            data.country.clone()
        };
        let key_blz = required_passport_value(
            &data.blz,
            "PinTAN passport has no bank code for SigHead KeyName",
        )?;
        let key_user_id = required_passport_value(
            &data.user_id,
            "PinTAN passport has no user id for SigHead KeyName",
        )?;
        let secfunc = passport
            .current_tan_method()
            .unwrap_or(ONESTEP_TAN_METHOD_ID)
            .to_owned();
        let profile_version = if secfunc == ONESTEP_TAN_METHOD_ID {
            "1"
        } else {
            "2"
        };

        Ok(Self {
            profile_method: "PIN".to_owned(),
            profile_version: profile_version.to_owned(),
            secfunc,
            seccheckref: seccheckref.into(),
            role: "1".to_owned(),
            sec_idn_func: "1".to_owned(),
            sys_id: "0".to_owned(),
            secref: secref.into(),
            timestamp_date: timestamp_date.into(),
            timestamp_time: timestamp_time.into(),
            hash_alg: "999".to_owned(),
            sig_alg: "10".to_owned(),
            sig_mode: "16".to_owned(),
            key_country,
            key_blz: key_blz.to_owned(),
            key_user_id: key_user_id.to_owned(),
            key_num: "0".to_owned(),
            key_version: "0".to_owned(),
        })
    }
}

pub fn apply_pintan_sig_head(
    message: &mut HbciMessage,
    sig_head_path: &str,
    sig_head: &PinTanSigHead,
) -> HbciResult<()> {
    message.set_value(
        &format!("{sig_head_path}.SecProfile.method"),
        &sig_head.profile_method,
    )?;
    message.set_value(
        &format!("{sig_head_path}.SecProfile.version"),
        &sig_head.profile_version,
    )?;
    message.set_value(&format!("{sig_head_path}.secfunc"), &sig_head.secfunc)?;
    message.set_value(
        &format!("{sig_head_path}.seccheckref"),
        &sig_head.seccheckref,
    )?;
    message.set_value(&format!("{sig_head_path}.role"), &sig_head.role)?;
    message.set_value(
        &format!("{sig_head_path}.SecIdnDetails.func"),
        &sig_head.sec_idn_func,
    )?;
    message.set_value(
        &format!("{sig_head_path}.SecIdnDetails.sysid"),
        &sig_head.sys_id,
    )?;
    message.set_value(&format!("{sig_head_path}.secref"), &sig_head.secref)?;
    message.set_value(
        &format!("{sig_head_path}.SecTimestamp.date"),
        &sig_head.timestamp_date,
    )?;
    message.set_value(
        &format!("{sig_head_path}.SecTimestamp.time"),
        &sig_head.timestamp_time,
    )?;
    message.set_value(&format!("{sig_head_path}.HashAlg.alg"), &sig_head.hash_alg)?;
    message.set_value(&format!("{sig_head_path}.SigAlg.alg"), &sig_head.sig_alg)?;
    message.set_value(&format!("{sig_head_path}.SigAlg.mode"), &sig_head.sig_mode)?;
    message.set_value(
        &format!("{sig_head_path}.KeyName.KIK.country"),
        &sig_head.key_country,
    )?;
    message.set_value(
        &format!("{sig_head_path}.KeyName.KIK.blz"),
        &sig_head.key_blz,
    )?;
    message.set_value(
        &format!("{sig_head_path}.KeyName.userid"),
        &sig_head.key_user_id,
    )?;
    message.set_value(
        &format!("{sig_head_path}.KeyName.keynum"),
        &sig_head.key_num,
    )?;
    message.set_value(
        &format!("{sig_head_path}.KeyName.keyversion"),
        &sig_head.key_version,
    )?;

    Ok(())
}

pub fn apply_pintan_sig_tail_from_head(
    message: &mut HbciMessage,
    sig_head_path: &str,
    sig_tail_path: &str,
) -> HbciResult<()> {
    let sig_head_checkref_path = format!("{sig_head_path}.seccheckref");
    let checkref = message
        .value(&sig_head_checkref_path)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("PinTAN SigHead has no check reference at {sig_head_checkref_path}"),
            )
        })?
        .to_owned();

    message.set_value(&format!("{sig_tail_path}.seccheckref"), &checkref)?;

    Ok(())
}

pub fn apply_pintan_signature_shell(
    message: &mut HbciMessage,
    sig_head_path: &str,
    sig_tail_path: &str,
    sig_head: &PinTanSigHead,
    signature: &[u8],
) -> HbciResult<()> {
    apply_pintan_sig_head(message, sig_head_path, sig_head)?;
    apply_pintan_sig_tail_from_head(message, sig_head_path, sig_tail_path)?;
    apply_pintan_user_sig_to_sig_tail(message, sig_tail_path, signature)
}

pub fn collect_pintan_signature_range(
    message: &HbciMessage,
    sig_head_path: &str,
    sig_tail_path: &str,
) -> HbciResult<String> {
    let children = message.root().children();
    let sig_head_index = top_level_child_index(children, sig_head_path)?;
    let sig_tail_index = top_level_child_index(children, sig_tail_path)?;

    if sig_tail_index <= sig_head_index {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("signature tail {sig_tail_path} does not follow head {sig_head_path}"),
        ));
    }

    let mut range = String::new();
    for child in &children[sig_head_index..sig_tail_index] {
        if let Some(rendered) = child.to_message_child_fints_string()? {
            range.push_str(&rendered);
        }
    }

    Ok(range)
}

pub fn apply_pintan_user_sig_to_sig_tail(
    message: &mut HbciMessage,
    sig_tail_path: &str,
    signature: &[u8],
) -> HbciResult<()> {
    let user_sig = UserSig::decode(Some(signature))?;

    message.set_value(&format!("{sig_tail_path}.UserSig.pin"), user_sig.pin())?;
    if !user_sig.tan().is_empty() {
        message.set_value(&format!("{sig_tail_path}.UserSig.tan"), user_sig.tan())?;
    }

    Ok(())
}

fn top_level_child_index(children: &[SyntaxElement], path: &str) -> HbciResult<usize> {
    children
        .iter()
        .position(|child| child.path() == path)
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("message has no top-level signature element {path}"),
            )
        })
}

fn random_seccheckref() -> String {
    let mut rng = OsRng;
    (rng.next_u32() & 0x7fff_ffff).to_string()
}

fn fints_timestamp_from_system_time(system_time: SystemTime) -> HbciResult<(String, String)> {
    let duration = system_time.duration_since(UNIX_EPOCH).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::InvalidArgument,
            "PinTAN signature timestamp is before the Unix epoch",
            err,
        )
    })?;
    let total_seconds = duration.as_secs();
    let days = (total_seconds / 86_400) as i64;
    let seconds_of_day = total_seconds % 86_400;
    let (year, month, day) = civil_date_from_unix_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    Ok((
        format!("{year:04}{month:02}{day:02}"),
        format!("{hour:02}{minute:02}{second:02}"),
    ))
}

fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };

    (year, month, day)
}

fn required_passport_value<'a>(value: &'a str, message: &str) -> HbciResult<&'a str> {
    if value.is_empty() {
        Err(HbciError::new(HbciErrorKind::InvalidArgument, message))
    } else {
        Ok(value)
    }
}
