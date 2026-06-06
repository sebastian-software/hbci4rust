use std::collections::BTreeMap;
use std::str;

use crate::callback::{CallbackDataType, CallbackEvent, CallbackReason};
use crate::comm::{CommClient, CommRequest, CommResponse, DefaultCommClient};
use crate::dialog::DialogContext;
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv::{HbciJob, JobRegistry};
use crate::gv_result::{
    GvrSaldoReq, GvrSaldoReqInfo, HbciDialogStatus, HbciExecStatus, HbciInstMessage, HbciJobResult,
    HbciJobResultData, HbciMsgStatus, HbciReturnValue, HbciStatus, Konto, Saldo, Value,
};
use crate::passport::PinTanPassport;
use crate::protocol::{HbciMessage, load_protocol_spec, parse_wire_message};

#[derive(Clone)]
pub struct HbciHandler<C = DefaultCommClient> {
    hbci_version: String,
    passport: PinTanPassport,
    comm: C,
    registry: JobRegistry,
    queue: Vec<HbciJob>,
    dialog: DialogContext,
    dialog_status: HbciDialogStatus,
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
            dialog: DialogContext::default(),
            dialog_status: HbciDialogStatus::default(),
        }
    }

    pub fn hbci_version(&self) -> &str {
        &self.hbci_version
    }

    pub fn passport(&self) -> &PinTanPassport {
        &self.passport
    }

    pub fn dialog_context(&self) -> &DialogContext {
        &self.dialog
    }

    pub fn dialog_status(&self) -> &HbciDialogStatus {
        &self.dialog_status
    }

    pub fn new_job(&self, name: &str) -> HbciResult<HbciJob> {
        self.registry.new_job(name)
    }

    pub fn add_to_queue(&mut self, job: HbciJob) {
        self.queue.push(job);
    }

    pub fn try_add_to_queue(&mut self, mut job: HbciJob) -> HbciResult<()> {
        job.verify_constraints()?;
        self.queue.push(job);
        Ok(())
    }

    pub async fn try_add_to_queue_with_account_checks(
        &mut self,
        mut job: HbciJob,
    ) -> HbciResult<()> {
        job.verify_constraints()?;
        let callback = super::callback();
        job.verify_account_checks(callback.as_deref()).await?;
        self.queue.push(job);
        Ok(())
    }

    pub fn queued_jobs(&self) -> &[HbciJob] {
        &self.queue
    }

    pub async fn init(&mut self) -> HbciResult<()> {
        let host = self
            .passport
            .host()
            .ok_or_else(|| {
                HbciError::new(
                    HbciErrorKind::InvalidArgument,
                    "PinTAN passport has no FinTS endpoint",
                )
            })?
            .to_owned();
        let request_ref = MessageReference::new("0", 1);
        let body = self.render_dialog_init(&request_ref)?;
        let callback = super::callback();

        if let Some(callback) = callback.as_ref() {
            callback
                .handle(CallbackEvent::new(CallbackReason::NeedConnection))
                .await?;
        }

        let response = self.comm.send(CommRequest::new(host, body)).await?;

        if let Some(callback) = callback.as_ref() {
            callback
                .handle(CallbackEvent::new(CallbackReason::CloseConnection))
                .await?;
        }

        if response.status >= 400 {
            return Err(HbciError::new(
                HbciErrorKind::Network,
                format!(
                    "FinTS dialog init failed with HTTP status {}",
                    response.status
                ),
            ));
        }

        let values = parse_dialog_init_response(&self.hbci_version, &response, &request_ref)?;
        let init_status = message_status_from_values(&values, "DialogInitRes");
        self.dialog = dialog_context_from_init_values(&values)?;
        self.dialog_status = HbciDialogStatus::new();
        self.dialog_status.set_init_status(init_status);
        self.passport
            .update_parameter_data_from_values(&values, "DialogInitRes");
        self.passport
            .update_accounts_from_values(&values, "DialogInitRes.UPD");
        if let Some(callback) = callback.as_ref() {
            for message in HbciInstMessage::collect_from_values(&values, "DialogInitRes.KIMsg") {
                callback
                    .handle(CallbackEvent {
                        reason: CallbackReason::HaveInstMsg,
                        message: message.to_string(),
                        data_type: CallbackDataType::None,
                        current_value: None,
                    })
                    .await?;
            }
        }
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

        let request_ref = MessageReference::new(
            self.dialog.current_dialog_id(),
            self.dialog.current_message_number(),
        );
        let body = self.render_queued_jobs(&request_ref)?;

        let response = self.comm.send(CommRequest::new(host, body)).await?;
        self.dialog.advance_message_number();
        let http_success = response.status < 400;
        let response_status = if http_success {
            parse_custom_message_response(&self.hbci_version, &response, &request_ref)?
        } else {
            ParsedResponseStatus::default()
        };
        let message_status = response_status.message_status();
        if self.dialog_status.init_status.is_some() {
            self.dialog_status.message_statuses.push(message_status);
        }
        let raw_response = Some(String::from_utf8_lossy(&response.body).into_owned());
        let global_status = response_status.global_status();

        let results = self
            .queue
            .drain(..)
            .enumerate()
            .map(|(index, job)| {
                let segment_sequence = queued_job_segment_sequence(index);
                let mut result_data = basic_result_data(&request_ref, segment_sequence);
                result_data.extend(response_status.result_data_for_job(&job, index));
                let mut result = HbciJobResult {
                    job_name: job.name().to_owned(),
                    raw_response: raw_response.clone(),
                    result_data,
                    global_return_values: response_status.global_return_values.clone(),
                    return_values: response_status.return_values_for_segment(segment_sequence),
                    result: response_status.result_for_job(&job, index, &self.passport),
                    success: false,
                };
                result.success = http_success && result.is_ok_with_global_status(&global_status);
                result
            })
            .collect::<Vec<_>>();
        let success =
            http_success && response_status.global_is_ok() && results.iter().all(|job| job.success);

        let mut exec_status = HbciExecStatus {
            success,
            job_results: results,
            messages: response_status.messages(),
            global_return_values: response_status.global_return_values,
            segment_return_values: response_status.segment_return_values,
            ..HbciExecStatus::default()
        };
        if self.dialog_status.init_status.is_some() {
            exec_status.add_dialog_status(
                self.customer_id_for_status(),
                Some(self.dialog_status.clone()),
            );
        }

        Ok(exec_status)
    }

    pub async fn close(&mut self) -> HbciResult<()> {
        if !self.dialog.is_open() {
            return Ok(());
        }

        let host = self
            .passport
            .host()
            .ok_or_else(|| {
                HbciError::new(
                    HbciErrorKind::InvalidArgument,
                    "PinTAN passport has no FinTS endpoint",
                )
            })?
            .to_owned();
        let dialog_id = self.dialog.open_dialog_id().ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                "DialogEnd requires an open FinTS dialog",
            )
        })?;
        let request_ref = MessageReference::new(dialog_id, self.dialog.current_message_number());
        let body = self.render_dialog_end(&request_ref)?;

        let response = self.comm.send(CommRequest::new(host, body)).await?;
        if response.status >= 400 {
            return Err(HbciError::new(
                HbciErrorKind::Network,
                format!(
                    "FinTS dialog end failed with HTTP status {}",
                    response.status
                ),
            ));
        }

        let end_status = parse_dialog_end_response(&self.hbci_version, &response, &request_ref)?;
        self.dialog_status.set_end_status(end_status.clone());
        ensure_dialog_end_ok(&end_status)?;
        self.dialog.reset();
        Ok(())
    }

    fn customer_id_for_status(&self) -> String {
        let passport = self.passport.data();
        passport
            .customer_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(&passport.user_id)
            .to_owned()
    }

    fn render_queued_jobs(&self, request_ref: &MessageReference) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg")?;

        message.set_value("CustomMsg.MsgHead.dialogid", &request_ref.dialog_id)?;
        message.set_value("CustomMsg.MsgHead.msgnum", &request_ref.msgnum)?;
        message.set_value("CustomMsg.MsgTail.msgnum", &request_ref.msgnum)?;

        for (index, job) in self.queue.iter().enumerate() {
            render_job_into_custom_message(&mut message, job, index, &self.passport)?;
        }

        message.prepare_outgoing()?;
        Ok(message.to_fints_string()?.into_bytes())
    }

    fn render_dialog_end(&self, request_ref: &MessageReference) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd")?;

        message.set_value("DialogEnd.MsgHead.dialogid", &request_ref.dialog_id)?;
        message.set_value("DialogEnd.MsgHead.msgnum", &request_ref.msgnum)?;
        message.set_value("DialogEnd.DialogEndS.dialogid", &request_ref.dialog_id)?;
        message.set_value("DialogEnd.MsgTail.msgnum", &request_ref.msgnum)?;

        message.prepare_outgoing()?;
        Ok(message.to_fints_string()?.into_bytes())
    }

    fn render_dialog_init(&self, request_ref: &MessageReference) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "DialogInit")?;
        let passport = self.passport.data();
        let country = if passport.country.is_empty() {
            "DE"
        } else {
            passport.country.as_str()
        };
        let blz = required_passport_value(&passport.blz, "PinTAN passport has no bank code")?;
        let customer_id = passport
            .customer_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(&passport.user_id);
        let customer_id =
            required_passport_value(customer_id, "PinTAN passport has no user id or customer id")?;

        message.set_value("DialogInit.MsgHead.dialogid", &request_ref.dialog_id)?;
        message.set_value("DialogInit.MsgHead.msgnum", &request_ref.msgnum)?;
        message.set_value("DialogInit.MsgTail.msgnum", &request_ref.msgnum)?;
        message.set_value("DialogInit.Idn.KIK.country", country)?;
        message.set_value("DialogInit.Idn.KIK.blz", blz)?;
        message.set_value("DialogInit.Idn.customerid", customer_id)?;
        message.set_value("DialogInit.Idn.sysid", "0")?;
        message.set_value("DialogInit.Idn.sysStatus", "0")?;
        message.set_value("DialogInit.ProcPrep.BPD", self.passport.bpd_version())?;
        message.set_value("DialogInit.ProcPrep.UPD", self.passport.upd_version())?;
        message.set_value("DialogInit.ProcPrep.lang", "0")?;
        message.set_value("DialogInit.ProcPrep.prodName", "hbci4rust")?;
        message.set_value(
            "DialogInit.ProcPrep.prodVersion",
            product_version_for_proc_prep(),
        )?;

        message.prepare_outgoing()?;
        Ok(message.to_fints_string()?.into_bytes())
    }
}

fn required_passport_value<'a>(value: &'a str, message: &str) -> HbciResult<&'a str> {
    if value.is_empty() {
        Err(HbciError::new(HbciErrorKind::InvalidArgument, message))
    } else {
        Ok(value)
    }
}

fn product_version_for_proc_prep() -> String {
    env!("CARGO_PKG_VERSION").chars().take(5).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MessageReference {
    dialog_id: String,
    msgnum: String,
}

impl MessageReference {
    fn new(dialog_id: impl Into<String>, msgnum: impl ToString) -> Self {
        Self {
            dialog_id: dialog_id.into(),
            msgnum: msgnum.to_string(),
        }
    }
}

fn render_job_into_custom_message(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    match job.name() {
        "KUmsAll" => render_kums_all(message, job, index, passport),
        "KUmsNew" => render_kums_new(message, job, index, passport),
        "SaldoReq" => render_saldo_request(message, job, index, passport),
        "SaldoReqAll" => render_saldo_request_all(message, job, index, passport),
        name => Err(HbciError::new(
            HbciErrorKind::Unsupported,
            format!("queued job rendering is not ported yet for {name}"),
        )),
    }
}

fn basic_result_data(
    request_ref: &MessageReference,
    segment_sequence: usize,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("basic.dialogid".to_owned(), request_ref.dialog_id.clone()),
        ("basic.msgnum".to_owned(), request_ref.msgnum.clone()),
        ("basic.segnum".to_owned(), segment_sequence.to_string()),
    ])
}

fn counted_result_header(base: &str, index: usize) -> String {
    if index == 0 {
        base.to_owned()
    } else {
        format!("{base}_{}", index + 1)
    }
}

fn saldo_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.SaldoRes7".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.SaldoRes7", index + 1)
    }
}

fn kums_response_root(segment_name: &str, index: usize) -> String {
    if index == 0 {
        format!("CustomMsgRes.GVRes.{segment_name}")
    } else {
        format!("CustomMsgRes.GVRes_{}.{segment_name}", index + 1)
    }
}

fn render_saldo_request(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_saldo_job(message, job, index, passport, "N", true)
}

fn render_saldo_request_all(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_saldo_job(message, job, index, passport, "J", false)
}

fn render_saldo_job(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
    default_allaccounts: &str,
    require_account: bool,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.Saldo7");
    let account = effective_job_account(job, passport, "Saldo7", "my");
    if require_account && !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "SaldoReq requires my.iban, my.number, or a passport account for the current Saldo7 tracer renderer",
        ));
    }

    set_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "Saldo7.allaccounts", "dummyall").unwrap_or(default_allaccounts),
    )?;
    if let Some(maxentries) = job_param(job, "Saldo7.maxentries", "maxentries") {
        message.set_value(&format!("{segment}.maxentries"), maxentries)?;
    }

    Ok(())
}

fn render_kums_all(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.KUmsZeit7");
    let account = effective_job_account(job, passport, "KUmsZeit7", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "KUmsAll requires my.iban, my.number, or a passport account for the current KUmsZeit7 tracer renderer",
        ));
    }

    set_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "KUmsZeit7.allaccounts", "dummy").unwrap_or("N"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "KUmsZeit7.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "KUmsZeit7.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "KUmsZeit7.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_kums_new(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.KUmsNew7");
    let account = effective_job_account(job, passport, "KUmsNew7", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "KUmsNew requires my.iban, my.number, or a passport account for the current KUmsNew7 tracer renderer",
        ));
    }

    set_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "KUmsNew7.allaccounts", "dummyall").unwrap_or("N"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "KUmsNew7.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn effective_job_account(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
    frontend_base: &str,
) -> Konto {
    let mut account = passport.first_account().cloned().unwrap_or_default();

    overlay_account_param(
        &mut account.iban,
        job_param(
            job,
            &format!("{lowlevel_segment}.KTV.iban"),
            &format!("{frontend_base}.iban"),
        ),
    );
    overlay_account_param(
        &mut account.bic,
        job_param(
            job,
            &format!("{lowlevel_segment}.KTV.bic"),
            &format!("{frontend_base}.bic"),
        ),
    );
    overlay_account_param(
        &mut account.country,
        job_param(
            job,
            &format!("{lowlevel_segment}.KTV.KIK.country"),
            &format!("{frontend_base}.country"),
        ),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(
            job,
            &format!("{lowlevel_segment}.KTV.KIK.blz"),
            &format!("{frontend_base}.blz"),
        ),
    );
    overlay_account_param(
        &mut account.number,
        job_param(
            job,
            &format!("{lowlevel_segment}.KTV.number"),
            &format!("{frontend_base}.number"),
        ),
    );
    overlay_account_param(
        &mut account.subnumber,
        job_param(
            job,
            &format!("{lowlevel_segment}.KTV.subnumber"),
            &format!("{frontend_base}.subnumber"),
        ),
    );

    account
}

fn job_param<'a>(job: &'a HbciJob, lowlevel_name: &str, frontend_name: &str) -> Option<&'a str> {
    job.lowlevel_param(lowlevel_name)
        .or_else(|| job.param(frontend_name))
        .filter(|value| !value.is_empty())
}

fn overlay_account_param(target: &mut Option<String>, value: Option<&str>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        *target = Some(value.to_owned());
    }
}

fn has_account_identity(account: &Konto) -> bool {
    account
        .iban
        .as_deref()
        .is_some_and(|value| !value.is_empty())
        || account
            .number
            .as_deref()
            .is_some_and(|value| !value.is_empty())
}

fn set_account_values(message: &mut HbciMessage, segment: &str, account: &Konto) -> HbciResult<()> {
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.iban"),
        account.iban.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.bic"),
        account.bic.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.KIK.country"),
        account.country.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.KIK.blz"),
        account.blz.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.number"),
        account.number.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.subnumber"),
        account.subnumber.as_deref(),
    )?;
    Ok(())
}

fn set_optional_message_value(
    message: &mut HbciMessage,
    path: &str,
    value: Option<&str>,
) -> HbciResult<()> {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        message.set_value(path, value)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct ParsedResponseStatus {
    global_return_values: Vec<HbciReturnValue>,
    segment_return_values: Vec<HbciReturnValue>,
    values: BTreeMap<String, String>,
}

impl ParsedResponseStatus {
    fn from_values(values: BTreeMap<String, String>) -> Self {
        let global_return_values =
            collect_return_values(&values, "CustomMsgRes.RetGlob", ReturnValueScope::Global);

        let mut segment_return_values = Vec::new();
        for prefix in counted_prefixes(&values, "CustomMsgRes.RetSeg") {
            segment_return_values.extend(collect_return_values(
                &values,
                &prefix,
                ReturnValueScope::Segment,
            ));
        }

        Self {
            global_return_values,
            segment_return_values,
            values,
        }
    }

    fn messages(&self) -> Vec<String> {
        self.global_return_values
            .iter()
            .chain(self.segment_return_values.iter())
            .map(HbciReturnValue::message)
            .collect()
    }

    fn global_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.global_return_values.clone())
    }

    fn segment_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.segment_return_values.clone())
    }

    fn message_status(&self) -> HbciMsgStatus {
        HbciMsgStatus::from_statuses(self.global_status(), self.segment_status())
    }

    fn global_is_ok(&self) -> bool {
        self.global_status().is_ok()
    }

    fn return_values_for_segment(&self, segment_sequence: usize) -> Vec<HbciReturnValue> {
        let segment_sequence = segment_sequence.to_string();
        self.segment_return_values
            .iter()
            .filter(|value| value.segment_ref.as_deref() == Some(segment_sequence.as_str()))
            .cloned()
            .collect()
    }

    fn result_for_job(
        &self,
        job: &HbciJob,
        index: usize,
        passport: &PinTanPassport,
    ) -> Option<HbciJobResultData> {
        match job.name() {
            "SaldoReq" => self
                .saldo_result_for_index(index, passport)
                .map(HbciJobResultData::SaldoReq),
            "SaldoReqAll" => {
                let result = self.saldo_result_all(passport);
                (!result.entries.is_empty()).then_some(HbciJobResultData::SaldoReq(result))
            }
            _ => None,
        }
    }

    fn result_data_for_job(&self, job: &HbciJob, index: usize) -> BTreeMap<String, String> {
        match job.name() {
            "KUmsAll" => self.content_result_data([kums_response_root("KUmsZeitRes7", index)]),
            "KUmsNew" => self.content_result_data([kums_response_root("KUmsNewRes7", index)]),
            "SaldoReq" => self.content_result_data([saldo_response_root(index)]),
            "SaldoReqAll" => self.content_result_data(
                counted_prefixes(&self.values, "CustomMsgRes.GVRes")
                    .into_iter()
                    .map(|prefix| format!("{prefix}.SaldoRes7")),
            ),
            _ => BTreeMap::new(),
        }
    }

    fn content_result_data<I>(&self, roots: I) -> BTreeMap<String, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut result_data = BTreeMap::new();

        for (index, root) in roots.into_iter().enumerate() {
            let content_header = counted_result_header("content", index);
            let root_prefix = format!("{root}.");
            for (key, value) in self
                .values
                .iter()
                .filter(|(key, _)| key.starts_with(&root_prefix))
            {
                let suffix = &key[root_prefix.len()..];
                result_data.insert(format!("{content_header}.{suffix}"), value.clone());
            }
        }

        result_data
    }

    fn saldo_result_for_index(
        &self,
        index: usize,
        passport: &PinTanPassport,
    ) -> Option<GvrSaldoReq> {
        let root = saldo_response_root(index);

        saldo_info_from_values(&self.values, &root, passport).map(|info| GvrSaldoReq {
            entries: vec![info],
        })
    }

    fn saldo_result_all(&self, passport: &PinTanPassport) -> GvrSaldoReq {
        let entries = counted_prefixes(&self.values, "CustomMsgRes.GVRes")
            .into_iter()
            .filter_map(|prefix| {
                saldo_info_from_values(&self.values, &format!("{prefix}.SaldoRes7"), passport)
            })
            .collect();

        GvrSaldoReq { entries }
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
    request_ref: &MessageReference,
) -> HbciResult<ParsedResponseStatus> {
    let values = parse_response_values(hbci_version, response, "CustomMsgRes")?;
    validate_response_message_reference(&values, "CustomMsgRes", request_ref)?;
    validate_open_dialog_response_id(&values, "CustomMsgRes", request_ref)?;

    Ok(ParsedResponseStatus::from_values(values))
}

fn parse_dialog_init_response(
    hbci_version: &str,
    response: &CommResponse,
    request_ref: &MessageReference,
) -> HbciResult<BTreeMap<String, String>> {
    let values = parse_response_values(hbci_version, response, "DialogInitRes")?;
    validate_response_message_reference(&values, "DialogInitRes", request_ref)?;

    Ok(values)
}

fn parse_dialog_end_response(
    hbci_version: &str,
    response: &CommResponse,
    request_ref: &MessageReference,
) -> HbciResult<HbciMsgStatus> {
    let values = parse_response_values(hbci_version, response, "DialogEndRes")?;
    validate_response_message_reference(&values, "DialogEndRes", request_ref)?;
    validate_open_dialog_response_id(&values, "DialogEndRes", request_ref)?;

    Ok(message_status_from_values(&values, "DialogEndRes"))
}

fn parse_response_values(
    hbci_version: &str,
    response: &CommResponse,
    message_name: &str,
) -> HbciResult<BTreeMap<String, String>> {
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

    resolved.values_for_message(&syntax, message_name)
}

fn validate_response_message_reference(
    values: &BTreeMap<String, String>,
    message_name: &str,
    expected: &MessageReference,
) -> HbciResult<()> {
    let actual = MessageReference {
        dialog_id: required_message_value(
            values,
            &format!("{message_name}.MsgHead.MsgRef.dialogid"),
            "FinTS response did not contain a message reference dialog id",
        )?
        .to_owned(),
        msgnum: required_message_value(
            values,
            &format!("{message_name}.MsgHead.MsgRef.msgnum"),
            "FinTS response did not contain a message reference number",
        )?
        .to_owned(),
    };

    if actual == *expected {
        return Ok(());
    }

    Err(HbciError::new(
        HbciErrorKind::Protocol,
        format!(
            "{message_name} references message {}:{}, expected {}:{}",
            actual.dialog_id, actual.msgnum, expected.dialog_id, expected.msgnum
        ),
    ))
}

fn validate_open_dialog_response_id(
    values: &BTreeMap<String, String>,
    message_name: &str,
    expected: &MessageReference,
) -> HbciResult<()> {
    if expected.dialog_id == "0" {
        return Ok(());
    }

    let actual = required_message_value(
        values,
        &format!("{message_name}.MsgHead.dialogid"),
        "FinTS response did not contain a dialog id",
    )?;

    if actual == expected.dialog_id {
        return Ok(());
    }

    Err(HbciError::new(
        HbciErrorKind::Protocol,
        format!(
            "{message_name} has dialog id {actual}, expected {}",
            expected.dialog_id
        ),
    ))
}

fn ensure_dialog_end_ok(status: &HbciMsgStatus) -> HbciResult<()> {
    if status.is_ok() {
        return Ok(());
    }

    let message = status
        .global_status
        .return_values
        .iter()
        .chain(status.segment_status.return_values.iter())
        .map(HbciReturnValue::message)
        .collect::<Vec<_>>()
        .join(", ");
    let message = if message.is_empty() {
        "FinTS dialog end failed without return values".to_owned()
    } else {
        format!("FinTS dialog end failed: {message}")
    };
    Err(HbciError::new(HbciErrorKind::Protocol, message))
}

fn message_status_from_values(
    values: &BTreeMap<String, String>,
    message_name: &str,
) -> HbciMsgStatus {
    let global_status = HbciStatus::from_return_values(collect_return_values(
        values,
        &format!("{message_name}.RetGlob"),
        ReturnValueScope::Global,
    ));
    let mut segment_return_values = Vec::new();
    for prefix in counted_prefixes(values, &format!("{message_name}.RetSeg")) {
        segment_return_values.extend(collect_return_values(
            values,
            &prefix,
            ReturnValueScope::Segment,
        ));
    }

    HbciMsgStatus::from_statuses(
        global_status,
        HbciStatus::from_return_values(segment_return_values),
    )
}

fn dialog_context_from_init_values(values: &BTreeMap<String, String>) -> HbciResult<DialogContext> {
    let dialog_id = required_message_value(
        values,
        "DialogInitRes.MsgHead.dialogid",
        "DialogInitRes did not contain a dialog id",
    )?;

    Ok(DialogContext::from_dialog_id(dialog_id))
}

fn saldo_info_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
    passport: &PinTanPassport,
) -> Option<GvrSaldoReqInfo> {
    values.get(&format!("{prefix}.SegHead.code"))?;

    let mut konto = Konto {
        country: optional_value(values, &format!("{prefix}.KTV.KIK.country")),
        blz: optional_value(values, &format!("{prefix}.KTV.KIK.blz")),
        number: optional_value(values, &format!("{prefix}.KTV.number")),
        subnumber: optional_value(values, &format!("{prefix}.KTV.subnumber")),
        bic: optional_value(values, &format!("{prefix}.KTV.bic")),
        iban: optional_value(values, &format!("{prefix}.KTV.iban")),
        customer_id: None,
        name: None,
        name2: None,
        acctype: None,
        account_type: optional_value(values, &format!("{prefix}.kontobez")),
        curr: optional_value(values, &format!("{prefix}.curr")),
        limit: None,
        allowed_gvs: Vec::new(),
    };
    passport.fill_account_info(&mut konto);
    let ready = saldo_from_values(values, &format!("{prefix}.booked"))?;

    Some(GvrSaldoReqInfo {
        konto,
        ready,
        unready: saldo_from_values(values, &format!("{prefix}.pending")),
        kredit: value_from_values(values, &format!("{prefix}.kredit")),
        available: value_from_values(values, &format!("{prefix}.available")),
        used: value_from_values(values, &format!("{prefix}.used")),
    })
}

fn saldo_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<Saldo> {
    let credit_debit = values.get(&format!("{prefix}.CreditDebit"))?;
    let amount = values
        .get(&format!("{prefix}.BTG.value"))
        .cloned()
        .unwrap_or_else(|| "0".to_owned());
    let value = Value {
        value: signed_amount(credit_debit, amount),
        curr: optional_value(values, &format!("{prefix}.BTG.curr")),
    };

    Some(Saldo {
        value,
        date: optional_value(values, &format!("{prefix}.date")),
        time: optional_value(values, &format!("{prefix}.time")),
    })
}

fn value_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<Value> {
    values.get(&format!("{prefix}.value")).map(|value| Value {
        value: value.to_owned(),
        curr: optional_value(values, &format!("{prefix}.curr")),
    })
}

fn signed_amount(credit_debit: &str, amount: String) -> String {
    if credit_debit == "D" {
        format!("-{amount}")
    } else {
        amount
    }
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

fn optional_value(values: &BTreeMap<String, String>, key: &str) -> Option<String> {
    values.get(key).and_then(|value| non_empty_string(value))
}

fn required_message_value<'a>(
    values: &'a BTreeMap<String, String>,
    key: &str,
    message: &str,
) -> HbciResult<&'a str> {
    values
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| HbciError::new(HbciErrorKind::Protocol, message))
}

fn queued_job_segment_sequence(index: usize) -> usize {
    index + 2
}
