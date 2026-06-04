use std::collections::BTreeMap;
use std::str;

use crate::comm::{CommClient, CommRequest, CommResponse, DefaultCommClient};
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv::{HbciJob, JobRegistry};
use crate::gv_result::{HbciExecStatus, HbciJobResult, HbciReturnValue};
use crate::passport::PinTanPassport;
use crate::protocol::{HbciMessage, load_protocol_spec, parse_wire_message};

#[derive(Clone)]
pub struct HbciHandler<C = DefaultCommClient> {
    hbci_version: String,
    passport: PinTanPassport,
    comm: C,
    registry: JobRegistry,
    queue: Vec<HbciJob>,
}

impl HbciHandler<DefaultCommClient> {
    pub fn new(hbci_version: impl Into<String>, passport: PinTanPassport) -> Self {
        Self::with_comm(hbci_version, passport, DefaultCommClient::default())
    }
}

impl<C> HbciHandler<C>
where
    C: CommClient,
{
    pub fn with_comm(hbci_version: impl Into<String>, passport: PinTanPassport, comm: C) -> Self {
        Self {
            hbci_version: hbci_version.into(),
            passport,
            comm,
            registry: JobRegistry::pintan(),
            queue: Vec::new(),
        }
    }

    pub fn hbci_version(&self) -> &str {
        &self.hbci_version
    }

    pub fn passport(&self) -> &PinTanPassport {
        &self.passport
    }

    pub fn new_job(&self, name: &str) -> HbciResult<HbciJob> {
        self.registry.new_job(name)
    }

    pub fn add_to_queue(&mut self, job: HbciJob) {
        self.queue.push(job);
    }

    pub fn queued_jobs(&self) -> &[HbciJob] {
        &self.queue
    }

    pub async fn init(&self) -> HbciResult<()> {
        let Some(callback) = super::callback() else {
            return Ok(());
        };

        callback
            .handle(crate::callback::CallbackEvent::new(
                crate::callback::CallbackReason::NeedConnection,
            ))
            .await?;
        callback
            .handle(crate::callback::CallbackEvent::new(
                crate::callback::CallbackReason::CloseConnection,
            ))
            .await?;
        Ok(())
    }

    pub async fn execute(&mut self) -> HbciResult<HbciExecStatus> {
        if self.queue.is_empty() {
            return Ok(HbciExecStatus::default());
        }

        let host = self.passport.host().ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                "PinTAN passport has no FinTS endpoint",
            )
        })?;

        let body = self.render_queued_jobs()?;

        let response = self.comm.send(CommRequest::new(host, body)).await?;
        let http_success = response.status < 400;
        let response_status = if http_success {
            parse_custom_message_response(&self.hbci_version, &response)?
        } else {
            ParsedResponseStatus::default()
        };
        let raw_response = Some(String::from_utf8_lossy(&response.body).into_owned());

        let results = self
            .queue
            .drain(..)
            .enumerate()
            .map(|(index, job)| {
                let segment_sequence = queued_job_segment_sequence(index);
                HbciJobResult {
                    job_name: job.name().to_owned(),
                    success: http_success && response_status.job_is_ok(segment_sequence),
                    raw_response: raw_response.clone(),
                    return_values: response_status.return_values_for_segment(segment_sequence),
                }
            })
            .collect::<Vec<_>>();
        let success =
            http_success && response_status.global_is_ok() && results.iter().all(|job| job.success);

        Ok(HbciExecStatus {
            success,
            job_results: results,
            messages: response_status.messages(),
            global_return_values: response_status.global_return_values,
            segment_return_values: response_status.segment_return_values,
        })
    }

    fn render_queued_jobs(&self) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg")?;

        message.set_value("CustomMsg.MsgHead.dialogid", "0")?;
        message.set_value("CustomMsg.MsgHead.msgnum", "1")?;
        message.set_value("CustomMsg.MsgTail.msgnum", "1")?;

        for (index, job) in self.queue.iter().enumerate() {
            render_job_into_custom_message(&mut message, job, index)?;
        }

        message.prepare_outgoing()?;
        Ok(message.to_fints_string()?.into_bytes())
    }
}

fn render_job_into_custom_message(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
) -> HbciResult<()> {
    match job.name() {
        "SaldoReq" => render_saldo_request(message, job, index),
        name => Err(HbciError::new(
            HbciErrorKind::Unsupported,
            format!("queued job rendering is not ported yet for {name}"),
        )),
    }
}

fn render_saldo_request(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.Saldo7");
    let iban = job.param("my.iban").ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::InvalidArgument,
            "SaldoReq requires my.iban for the current Saldo7 tracer renderer",
        )
    })?;

    message.set_value(&format!("{segment}.KTV.iban"), iban)?;
    if let Some(bic) = job.param("my.bic") {
        message.set_value(&format!("{segment}.KTV.bic"), bic)?;
    }
    message.set_value(
        &format!("{segment}.allaccounts"),
        job.param("dummyall").unwrap_or("N"),
    )?;
    if let Some(maxentries) = job.param("maxentries") {
        message.set_value(&format!("{segment}.maxentries"), maxentries)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Default)]
struct ParsedResponseStatus {
    global_return_values: Vec<HbciReturnValue>,
    segment_return_values: Vec<HbciReturnValue>,
}

impl ParsedResponseStatus {
    fn from_values(values: &BTreeMap<String, String>) -> Self {
        let global_return_values =
            collect_return_values(values, "CustomMsgRes.RetGlob", ReturnValueScope::Global);

        let mut segment_return_values = Vec::new();
        for prefix in counted_prefixes(values, "CustomMsgRes.RetSeg") {
            segment_return_values.extend(collect_return_values(
                values,
                &prefix,
                ReturnValueScope::Segment,
            ));
        }

        Self {
            global_return_values,
            segment_return_values,
        }
    }

    fn messages(&self) -> Vec<String> {
        self.global_return_values
            .iter()
            .chain(self.segment_return_values.iter())
            .map(HbciReturnValue::message)
            .collect()
    }

    fn global_is_ok(&self) -> bool {
        status_is_ok(&self.global_return_values)
    }

    fn job_is_ok(&self, segment_sequence: usize) -> bool {
        let segment_sequence = segment_sequence.to_string();
        let job_return_values = self
            .segment_return_values
            .iter()
            .filter(|value| value.segment_ref.as_deref() == Some(segment_sequence.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        status_pair_is_ok(&self.global_return_values, &job_return_values)
    }

    fn return_values_for_segment(&self, segment_sequence: usize) -> Vec<HbciReturnValue> {
        let segment_sequence = segment_sequence.to_string();
        self.segment_return_values
            .iter()
            .filter(|value| value.segment_ref.as_deref() == Some(segment_sequence.as_str()))
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, Copy)]
enum ReturnValueScope {
    Global,
    Segment,
}

fn parse_custom_message_response(
    hbci_version: &str,
    response: &CommResponse,
) -> HbciResult<ParsedResponseStatus> {
    let body = str::from_utf8(&response.body).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            "FinTS response body is not UTF-8 text",
            err,
        )
    })?;
    let syntax = load_protocol_spec(hbci_version)?.parse_syntax()?;
    let wire_message = parse_wire_message(body)?;
    let resolved = wire_message.resolve_segments(&syntax)?;
    let values = resolved.values_for_message(&syntax, "CustomMsgRes")?;

    Ok(ParsedResponseStatus::from_values(&values))
}

fn collect_return_values(
    values: &BTreeMap<String, String>,
    container_prefix: &str,
    scope: ReturnValueScope,
) -> Vec<HbciReturnValue> {
    counted_prefixes(values, &format!("{container_prefix}.RetVal"))
        .into_iter()
        .filter_map(|prefix| return_value(values, &prefix, scope))
        .collect()
}

fn return_value(
    values: &BTreeMap<String, String>,
    prefix: &str,
    scope: ReturnValueScope,
) -> Option<HbciReturnValue> {
    let code = values.get(&format!("{prefix}.code"))?.to_owned();
    let ret_ref = values
        .get(&format!("{prefix}.ref"))
        .and_then(|value| non_empty_string(value));
    let text = values
        .get(&format!("{prefix}.text"))
        .cloned()
        .unwrap_or_default();
    let mut params = Vec::new();
    for param_prefix in counted_value_keys(values, &format!("{prefix}.parm")) {
        if let Some(param) = values
            .get(&param_prefix)
            .and_then(|value| non_empty_string(value))
        {
            params.push(param);
        }
    }

    let mut value = HbciReturnValue::new(code, text);
    match scope {
        ReturnValueScope::Global => {
            value.data_ref = ret_ref;
        }
        ReturnValueScope::Segment => {
            value.segment_ref = ret_ref;
        }
    }
    value.params = params;

    Some(value)
}

fn counted_prefixes(values: &BTreeMap<String, String>, base: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut index = 1;

    loop {
        let prefix = if index == 1 {
            base.to_owned()
        } else {
            format!("{base}_{index}")
        };
        let child_prefix = format!("{prefix}.");
        if !values.keys().any(|key| key.starts_with(&child_prefix)) {
            break;
        }

        prefixes.push(prefix);
        index += 1;
    }

    prefixes
}

fn counted_value_keys(values: &BTreeMap<String, String>, base: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut index = 1;

    loop {
        let key = if index == 1 {
            base.to_owned()
        } else {
            format!("{base}_{index}")
        };
        if !values.contains_key(&key) {
            break;
        }

        keys.push(key);
        index += 1;
    }

    keys
}

fn non_empty_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

fn queued_job_segment_sequence(index: usize) -> usize {
    index + 2
}

fn status_is_ok(values: &[HbciReturnValue]) -> bool {
    !values.iter().any(HbciReturnValue::is_error)
        && values.iter().any(HbciReturnValue::is_known_status)
}

fn status_pair_is_ok(global_values: &[HbciReturnValue], job_values: &[HbciReturnValue]) -> bool {
    !global_values.iter().any(HbciReturnValue::is_error)
        && !job_values.iter().any(HbciReturnValue::is_error)
        && (global_values.iter().any(HbciReturnValue::is_known_status)
            || job_values.iter().any(HbciReturnValue::is_known_status))
}
