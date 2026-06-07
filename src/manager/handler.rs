use std::collections::BTreeMap;
use std::str;

use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::callback::{CallbackDataType, CallbackEvent, CallbackReason, HbciCallback};
use crate::comm::{CommClient, CommRequest, CommResponse, DefaultCommClient};
use crate::dialog::DialogContext;
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv::{CLASSIC_USAGE_LINE_COUNT, HbciJob, JobRegistry};
use crate::gv_result::{
    GvrAccInfo, GvrAccInfoAddress, GvrAccInfoEntry, GvrCardInfo, GvrCardList, GvrDauerEdit,
    GvrDauerList, GvrDauerListAussetzung, GvrDauerListEntry, GvrDauerNew, GvrFestCond,
    GvrFestCondList, GvrFestList, GvrFestListEntry, GvrFestListProlong, GvrInfoList,
    GvrInfoListInfo, GvrInfoOrder, GvrInfoOrderInfo, GvrInstUebSepa, GvrKUms, GvrKontoauszug,
    GvrKontoauszugEntry, GvrLastSepa, GvrSaldoReq, GvrSaldoReqInfo, GvrStatus, GvrStatusEntry,
    GvrTanInfo, GvrTanList, GvrTanListEntry, GvrTanMediaInfo, GvrTanMediaList, GvrTermUeb,
    GvrTermUebEdit, GvrTermUebList, GvrTermUebListEntry, GvrVoP, GvrWPDepotList, GvrWPDepotUms,
    HbciDialogStatus, HbciExecStatus, HbciInstMessage, HbciJobResult, HbciJobResultData,
    HbciMsgStatus, HbciReturnValue, HbciStatus, Konto, KontoauszugFormat, Saldo, Value, VoPResult,
    VoPResultItem, VoPStatus,
};
use crate::passport::{
    ONESTEP_TAN_METHOD_ID, PinTanPassport, TanMethodOption, TanMethodSelection, UserSig,
};
use crate::protocol::{HbciMessage, load_protocol_spec, parse_wire_message};
use crate::sepa::{
    CAMT_052_001_01_URN, PAIN_001_001_02_URN, PAIN_008_001_01_URN, PAIN_008_001_02_URN,
    parse_pain_001_transfers, parse_pain_008_direct_debits,
};
use crate::swift::decode_umlauts;
use crate::tools::Properties;

use super::{
    ChallengeInfo, OrderHashMode, PinTanSignatureContext, apply_pintan_sig_head,
    apply_pintan_sig_tail_from_head, apply_pintan_user_sig_to_sig_tail,
    collect_pintan_segment_codes, collect_pintan_signature_range,
};

const CLASSIC_INLAND_USER4_SNAPSHOT_SUFFIXES: &[&str] = &[
    "My.number",
    "My.subnumber",
    "My.KIK.country",
    "My.KIK.blz",
    "Other.number",
    "Other.subnumber",
    "Other.KIK.country",
    "Other.KIK.blz",
    "name",
    "name2",
    "BTG.value",
    "BTG.curr",
    "key",
    "addkey",
    "usage.usage",
    "usage.usage_2",
    "usage.usage_3",
    "usage.usage_4",
    "usage.usage_5",
    "usage.usage_6",
    "usage.usage_7",
    "usage.usage_8",
    "usage.usage_9",
    "usage.usage_10",
    "usage.usage_11",
    "usage.usage_12",
    "usage.usage_13",
    "usage.usage_14",
    "date",
];

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

    pub fn new_tan2step_initial_job(
        &self,
        task: &HbciJob,
        challenge_info: Option<&ChallengeInfo>,
    ) -> HbciResult<HbciJob> {
        let tan_media = self.passport.tan_media_for_hktan_without_callback();
        new_tan2step_initial_job(
            &self.registry,
            &self.hbci_version,
            &self.passport,
            task,
            tan_media.as_deref(),
            challenge_info,
        )
    }

    pub async fn new_tan2step_initial_job_with_tan_media_selection(
        &mut self,
        task: &HbciJob,
        challenge_info: Option<&ChallengeInfo>,
    ) -> HbciResult<HbciJob> {
        let callback = super::callback();
        let tan_media = choose_tan_media_if_needed(&mut self.passport, callback.as_deref()).await?;
        new_tan2step_initial_job(
            &self.registry,
            &self.hbci_version,
            &self.passport,
            task,
            tan_media.as_deref(),
            challenge_info,
        )
    }

    pub fn new_tan2step_process1_job(
        &self,
        task: &HbciJob,
        challenge_info: Option<&ChallengeInfo>,
    ) -> HbciResult<HbciJob> {
        let tan_media = self.passport.tan_media_for_hktan_without_callback();
        new_tan2step_process1_job(
            &self.registry,
            &self.hbci_version,
            &self.passport,
            task,
            tan_media.as_deref(),
            challenge_info,
        )
    }

    pub async fn new_tan2step_process1_job_with_tan_media_selection(
        &mut self,
        task: &HbciJob,
        challenge_info: Option<&ChallengeInfo>,
    ) -> HbciResult<HbciJob> {
        let callback = super::callback();
        let tan_media = choose_tan_media_if_needed(&mut self.passport, callback.as_deref()).await?;
        new_tan2step_process1_job(
            &self.registry,
            &self.hbci_version,
            &self.passport,
            task,
            tan_media.as_deref(),
            challenge_info,
        )
    }

    pub fn new_tan2step_process2_step1_job(&self, task: &HbciJob) -> HbciResult<HbciJob> {
        let tan_media = self.passport.tan_media_for_hktan_without_callback();
        new_tan2step_process2_step1_job(&self.registry, task, tan_media.as_deref())
    }

    pub async fn new_tan2step_process2_step1_job_with_tan_media_selection(
        &mut self,
        task: &HbciJob,
    ) -> HbciResult<HbciJob> {
        let callback = super::callback();
        let tan_media = choose_tan_media_if_needed(&mut self.passport, callback.as_deref()).await?;
        new_tan2step_process2_step1_job(&self.registry, task, tan_media.as_deref())
    }

    pub fn new_tan2step_process2_job(&self) -> HbciResult<HbciJob> {
        new_tan2step_process2_job(&self.registry, &self.passport)
    }

    pub async fn request_tan_for_sca(&self) -> HbciResult<Option<String>> {
        let callback = super::callback();
        request_tan_for_sca(&self.passport, callback.as_deref()).await
    }

    pub async fn request_pin(&mut self) -> HbciResult<String> {
        let callback = super::callback();
        request_pin(&mut self.passport, callback.as_deref()).await
    }

    pub async fn sign_pintan_user_sig_for_sca(&mut self) -> HbciResult<Vec<u8>> {
        let callback = super::callback();
        sign_pintan_user_sig_for_sca(&mut self.passport, callback.as_deref()).await
    }

    pub fn add_to_queue(&mut self, job: HbciJob) {
        self.queue.push(job);
    }

    pub fn try_add_to_queue(&mut self, mut job: HbciJob) -> HbciResult<()> {
        self.prepare_job_from_passport(&mut job)?;
        job.verify_constraints()?;
        self.queue.push(job);
        Ok(())
    }

    pub fn try_add_to_queue_with_initial_tan_job(&mut self, mut job: HbciJob) -> HbciResult<()> {
        self.prepare_job_from_passport(&mut job)?;
        job.verify_constraints()?;
        let hktan = self.initial_tan_job_for_queue(&job)?;
        self.queue.push(job);
        if let Some(hktan) = hktan {
            self.queue.push(hktan);
        }
        Ok(())
    }

    pub async fn try_add_to_queue_with_account_checks(
        &mut self,
        mut job: HbciJob,
    ) -> HbciResult<()> {
        self.prepare_job_from_passport(&mut job)?;
        job.verify_constraints()?;
        let callback = super::callback();
        job.verify_account_checks(callback.as_deref()).await?;
        self.queue.push(job);
        Ok(())
    }

    fn prepare_job_from_passport(&self, job: &mut HbciJob) -> HbciResult<()> {
        match job.name() {
            "DauerDel" => apply_dauer_snapshot_to_job(job, &self.passport, "DauerDel4"),
            "DauerEdit" => apply_dauer_snapshot_to_job(job, &self.passport, "DauerEdit5"),
            "TermUebDel" => apply_term_ueb_snapshot_to_job(job, &self.passport, "TermUebDel3"),
            "TermUebEdit" => apply_term_ueb_snapshot_to_job(job, &self.passport, "TermUebEdit4"),
            _ => Ok(()),
        }
    }

    pub fn queued_jobs(&self) -> &[HbciJob] {
        &self.queue
    }

    fn initial_tan_job_for_queue(&self, task: &HbciJob) -> HbciResult<Option<HbciJob>> {
        if task.name() == "TAN2Step" {
            return Ok(None);
        }
        if self
            .passport
            .current_tan_method()
            .unwrap_or(ONESTEP_TAN_METHOD_ID)
            == ONESTEP_TAN_METHOD_ID
        {
            return Ok(None);
        }

        let task_info = orderhash_source_job_info(task.name())?;
        if self
            .passport
            .pin_tan_info_for_segment_code(task_info.code)
            .as_deref()
            != Some("J")
        {
            return Ok(None);
        }
        if self.passport.tan2step_parameter("process").as_deref() == Some("1") {
            return Err(HbciError::new(
                HbciErrorKind::Unsupported,
                "process-1 automatic HKTAN queueing requires multi-message execution",
            ));
        }

        let tan_media = self.passport.tan_media_for_hktan_without_callback();
        let mut hktan =
            new_tan2step_process2_step1_job(&self.registry, task, tan_media.as_deref())?;
        hktan.verify_constraints()?;
        Ok(Some(hktan))
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
        let callback = super::callback();
        let body = self
            .render_dialog_init(&request_ref, callback.as_deref())
            .await?;

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
        if let Some(init_status) = self.dialog_status.init_status.as_ref() {
            self.passport
                .update_allowed_twostep_mechanisms_from_status(init_status);
        }
        self.passport
            .update_parameter_data_from_values(&values, "DialogInitRes");
        let selection = self.passport.determine_tan_method();
        if let Some(selected) = choose_tan_method_if_needed(selection, callback.as_deref()).await? {
            self.passport.set_current_tan_method(selected);
        }
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

        let request_ref = MessageReference::new(
            self.dialog.current_dialog_id(),
            self.dialog.current_message_number(),
        );
        let callback = super::callback();
        let body = self
            .render_queued_jobs(&request_ref, callback.as_deref())
            .await?;

        let response = self.comm.send(CommRequest::new(host, body)).await?;
        self.dialog.advance_message_number();
        let http_success = response.status < 400;
        let response_status = if http_success {
            parse_custom_message_response(&self.hbci_version, &response, &request_ref)?
        } else {
            ParsedResponseStatus::default()
        };
        let message_status = response_status.message_status();
        self.passport.update_sca_state_from_response_values(
            &response_status.values,
            "CustomMsgRes",
            &message_status,
        );
        if http_success {
            update_passport_accounts_from_sepa_info(
                &mut self.passport,
                &self.queue,
                &response_status,
            );
        }
        if self.dialog_status.init_status.is_some() {
            self.dialog_status.message_statuses.push(message_status);
        }
        let raw_response = Some(String::from_utf8_lossy(&response.body).into_owned());
        let global_status = response_status.global_status();

        let queued_jobs = self.queue.clone();
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
        update_passport_tan_media_names_from_results(&mut self.passport, &results);
        update_passport_job_persistent_data_from_results(
            &mut self.passport,
            &queued_jobs,
            &results,
        );
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

    pub async fn execute_tan2step_process2_submission(&mut self) -> HbciResult<HbciExecStatus> {
        if !self.queue.is_empty() {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "process-2 TAN submission requires an empty queue",
            ));
        }

        let hktan = self.new_tan2step_process2_job()?;
        self.try_add_to_queue(hktan)?;
        let status = self.execute().await?;
        if status.success {
            self.passport.clear_sca_state();
        }
        Ok(status)
    }

    pub async fn execute_with_tan2step_process2(&mut self) -> HbciResult<HbciExecStatus> {
        let mut status = self.execute().await?;
        if self.should_execute_tan2step_process2_submission(&status) {
            let submission_status = self.execute_tan2step_process2_submission().await?;
            merge_exec_status(&mut status, submission_status);
        }
        Ok(status)
    }

    fn should_execute_tan2step_process2_submission(&self, status: &HbciExecStatus) -> bool {
        status.success
            && self
                .passport
                .current_tan_method()
                .unwrap_or(ONESTEP_TAN_METHOD_ID)
                != ONESTEP_TAN_METHOD_ID
            && self.passport.tan2step_parameter("process").as_deref() != Some("1")
            && self
                .passport
                .sca_state()
                .order_ref
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }

    pub async fn execute_with_tan2step_process1(
        &mut self,
        mut job: HbciJob,
    ) -> HbciResult<HbciExecStatus> {
        if !self.queue.is_empty() {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "process-1 execution requires an empty queue",
            ));
        }
        if self
            .passport
            .current_tan_method()
            .unwrap_or(ONESTEP_TAN_METHOD_ID)
            == ONESTEP_TAN_METHOD_ID
        {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "process-1 execution requires a two-step TAN method",
            ));
        }
        if self.passport.tan2step_parameter("process").as_deref() != Some("1") {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "process-1 execution requires BPD process=1",
            ));
        }

        self.prepare_job_from_passport(&mut job)?;
        job.verify_constraints()?;
        let mut hktan = self.new_tan2step_process1_job(&job, None)?;
        hktan.verify_constraints()?;
        self.queue.push(hktan);

        let mut status = self.execute().await?;
        if status.success {
            self.queue.push(job);
            let order_status = self.execute().await?;
            if order_status.success {
                self.passport.clear_sca_state();
            }
            merge_exec_status(&mut status, order_status);
        }
        Ok(status)
    }

    pub async fn execute_with_tan2step(&mut self) -> HbciResult<HbciExecStatus> {
        if self.queue.is_empty()
            || self
                .passport
                .current_tan_method()
                .unwrap_or(ONESTEP_TAN_METHOD_ID)
                == ONESTEP_TAN_METHOD_ID
            || !self.queue_contains_hktan_required_job()?
        {
            return self.execute().await;
        }

        if self.passport.tan2step_parameter("process").as_deref() == Some("1") {
            if self.queue.len() != 1 {
                return Err(HbciError::new(
                    HbciErrorKind::Unsupported,
                    "process-1 dispatcher currently supports exactly one TAN-required queued job",
                ));
            }
            let job = self.queue.remove(0);
            return self.execute_with_tan2step_process1(job).await;
        }

        if self.queue.len() == 1 {
            let job = self.queue.remove(0);
            self.try_add_to_queue_with_initial_tan_job(job)?;
        }
        self.execute_with_tan2step_process2().await
    }

    fn queue_contains_hktan_required_job(&self) -> HbciResult<bool> {
        self.queue.iter().try_fold(false, |found, job| {
            Ok(found || self.job_requires_hktan(job)?)
        })
    }

    fn job_requires_hktan(&self, job: &HbciJob) -> HbciResult<bool> {
        if job.name() == "TAN2Step" {
            return Ok(false);
        }

        let task_info = orderhash_source_job_info(job.name())?;
        Ok(self
            .passport
            .pin_tan_info_for_segment_code(task_info.code)
            .as_deref()
            == Some("J"))
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
        let callback = super::callback();
        let body = self
            .render_dialog_end(&request_ref, callback.as_deref())
            .await?;

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

    async fn render_queued_jobs(
        &mut self,
        request_ref: &MessageReference,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg")?;

        message.set_value("CustomMsg.MsgHead.dialogid", &request_ref.dialog_id)?;
        message.set_value("CustomMsg.MsgHead.msgnum", &request_ref.msgnum)?;
        message.set_value("CustomMsg.MsgTail.msgnum", &request_ref.msgnum)?;

        for (index, job) in self.queue.iter().enumerate() {
            render_job_into_custom_message(&mut message, job, index, &self.passport)?;
        }

        apply_pintan_signature(
            &mut message,
            "CustomMsg.SigHead",
            "CustomMsg.SigTail",
            &mut self.passport,
            callback,
        )
        .await?;

        message.prepare_outgoing()?;
        message.to_fints_bytes()
    }

    async fn render_dialog_end(
        &mut self,
        request_ref: &MessageReference,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd")?;

        message.set_value("DialogEnd.MsgHead.dialogid", &request_ref.dialog_id)?;
        message.set_value("DialogEnd.MsgHead.msgnum", &request_ref.msgnum)?;
        message.set_value("DialogEnd.DialogEndS.dialogid", &request_ref.dialog_id)?;
        message.set_value("DialogEnd.MsgTail.msgnum", &request_ref.msgnum)?;

        apply_pintan_signature(
            &mut message,
            "DialogEnd.SigHead",
            "DialogEnd.SigTail",
            &mut self.passport,
            callback,
        )
        .await?;

        message.prepare_outgoing()?;
        message.to_fints_bytes()
    }

    async fn render_dialog_init(
        &mut self,
        request_ref: &MessageReference,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<Vec<u8>> {
        let syntax = load_protocol_spec(&self.hbci_version)?.parse_syntax()?;
        let mut message = HbciMessage::from_syntax(&syntax, "DialogInit")?;
        let passport = self.passport.data();
        let country = if passport.country.is_empty() {
            "DE".to_owned()
        } else {
            passport.country.clone()
        };
        let blz =
            required_passport_value(&passport.blz, "PinTAN passport has no bank code")?.to_owned();
        let customer_id = passport
            .customer_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(&passport.user_id);
        let customer_id =
            required_passport_value(customer_id, "PinTAN passport has no user id or customer id")?
                .to_owned();
        let bpd_version = self.passport.bpd_version().to_owned();
        let upd_version = self.passport.upd_version().to_owned();

        message.set_value("DialogInit.MsgHead.dialogid", &request_ref.dialog_id)?;
        message.set_value("DialogInit.MsgHead.msgnum", &request_ref.msgnum)?;
        message.set_value("DialogInit.MsgTail.msgnum", &request_ref.msgnum)?;
        message.set_value("DialogInit.Idn.KIK.country", country)?;
        message.set_value("DialogInit.Idn.KIK.blz", blz)?;
        message.set_value("DialogInit.Idn.customerid", customer_id)?;
        message.set_value("DialogInit.Idn.sysid", "0")?;
        message.set_value("DialogInit.Idn.sysStatus", "0")?;
        message.set_value("DialogInit.ProcPrep.BPD", bpd_version)?;
        message.set_value("DialogInit.ProcPrep.UPD", upd_version)?;
        message.set_value("DialogInit.ProcPrep.lang", "0")?;
        message.set_value("DialogInit.ProcPrep.prodName", "hbci4rust")?;
        message.set_value(
            "DialogInit.ProcPrep.prodVersion",
            product_version_for_proc_prep(),
        )?;

        apply_pintan_signature(
            &mut message,
            "DialogInit.SigHead",
            "DialogInit.SigTail",
            &mut self.passport,
            callback,
        )
        .await?;

        message.prepare_outgoing()?;
        message.to_fints_bytes()
    }
}

fn merge_exec_status(target: &mut HbciExecStatus, mut source: HbciExecStatus) {
    target.success = target.success && source.success;
    target.job_results.append(&mut source.job_results);
    target.messages.append(&mut source.messages);
    target
        .global_return_values
        .append(&mut source.global_return_values);
    target
        .segment_return_values
        .append(&mut source.segment_return_values);

    for (customer_id, status) in source.dialog_statuses {
        if let Some(existing) = target.dialog_statuses.get_mut(&customer_id) {
            merge_dialog_status(existing, status);
        } else {
            target.dialog_statuses.insert(customer_id, status);
        }
    }
    for (customer_id, mut messages) in source.exception_messages {
        target
            .exception_messages
            .entry(customer_id)
            .or_default()
            .append(&mut messages);
    }
}

fn merge_dialog_status(target: &mut HbciDialogStatus, mut source: HbciDialogStatus) {
    if target.init_status.is_none() {
        target.init_status = source.init_status.take();
    }

    let common_prefix_len = target
        .message_statuses
        .iter()
        .zip(source.message_statuses.iter())
        .take_while(|(left, right)| left == right)
        .count();
    target
        .message_statuses
        .extend(source.message_statuses.into_iter().skip(common_prefix_len));

    if source.end_status.is_some() {
        target.end_status = source.end_status;
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

async fn choose_tan_method_if_needed(
    selection: TanMethodSelection,
    callback: Option<&dyn HbciCallback>,
) -> HbciResult<Option<String>> {
    let TanMethodSelection::NeedsUserSelection(options) = selection else {
        return Ok(None);
    };
    let Some(callback) = callback else {
        return Ok(None);
    };

    choose_tan_method(callback, &options).await.map(Some)
}

async fn choose_tan_method(
    callback: &dyn HbciCallback,
    options: &[TanMethodOption],
) -> HbciResult<String> {
    let response = callback
        .handle(CallbackEvent {
            reason: CallbackReason::NeedPtSecMech,
            message: "*** Select a pintan method from the list".to_owned(),
            data_type: CallbackDataType::Select,
            current_value: Some(format_tan_method_options(options)),
        })
        .await?;
    let Some(selected) = response.value.filter(|value| !value.is_empty()) else {
        return Err(HbciError::new(
            HbciErrorKind::Callback,
            "callback did not select a pintan method",
        ));
    };

    if options.iter().any(|option| option.id == selected) {
        Ok(selected)
    } else {
        Err(HbciError::new(
            HbciErrorKind::Callback,
            format!("selected pintan method not supported: {selected}"),
        ))
    }
}

fn format_tan_method_options(options: &[TanMethodOption]) -> String {
    options
        .iter()
        .map(|option| format!("{}:{}", option.id, option.name.as_deref().unwrap_or("null")))
        .collect::<Vec<_>>()
        .join("|")
}

async fn choose_tan_media_if_needed(
    passport: &mut PinTanPassport,
    callback: Option<&dyn HbciCallback>,
) -> HbciResult<Option<String>> {
    if let Some(tan_media) = passport.tan_media().filter(|value| !value.is_empty()) {
        return Ok(Some(tan_media.to_owned()));
    }
    if !passport.tan_media_required() {
        return Ok(None);
    }

    if let Some(callback) = callback {
        let response = callback
            .handle(CallbackEvent {
                reason: CallbackReason::NeedPtTanMedia,
                message: "*** Enter the name of your TAN media".to_owned(),
                data_type: CallbackDataType::Text,
                current_value: Some(passport.tan_media_names_value()),
            })
            .await?;
        if let Some(tan_media) = response.value.filter(|value| !value.trim().is_empty()) {
            passport.set_tan_media(tan_media.clone());
            return Ok(Some(tan_media));
        }
    }

    Ok(Some("noref".to_owned()))
}

async fn request_tan_for_sca(
    passport: &PinTanPassport,
    callback: Option<&dyn HbciCallback>,
) -> HbciResult<Option<String>> {
    let sca = passport.sca_state();
    let Some(challenge) = sca.challenge.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if sca.sca_exempted {
        return Ok(None);
    }

    let secmech = passport.current_secmech_info();
    let tan = request_tan_value(
        callback,
        format_sca_challenge_message(&secmech, challenge),
        sca.hhd_uc.clone(),
    )
    .await?;

    Ok(Some(tan))
}

async fn request_tan_for_one_step(
    passport: &PinTanPassport,
    callback: Option<&dyn HbciCallback>,
    signed_range: Option<&str>,
) -> HbciResult<Option<String>> {
    let Some(signed_range) = signed_range else {
        return Ok(None);
    };

    for code in collect_pintan_segment_codes(signed_range)? {
        if passport.pin_tan_info_for_segment_code(&code).as_deref() == Some("J") {
            let tan =
                request_tan_value(callback, "Please enter a TAN now".to_owned(), None).await?;
            return Ok(Some(tan));
        }
    }

    Ok(None)
}

async fn request_tan_value(
    callback: Option<&dyn HbciCallback>,
    message: String,
    current_value: Option<String>,
) -> HbciResult<String> {
    let Some(callback) = callback else {
        return Err(HbciError::new(
            HbciErrorKind::Callback,
            "callback required for TAN challenge",
        ));
    };

    let response = callback
        .handle(CallbackEvent {
            reason: CallbackReason::NeedPtTan,
            message,
            data_type: CallbackDataType::Text,
            current_value,
        })
        .await?;
    let Some(tan) = response.value.filter(|value| !value.trim().is_empty()) else {
        return Err(HbciError::new(
            HbciErrorKind::Callback,
            "callback did not provide a TAN",
        ));
    };

    Ok(tan)
}

fn format_sca_challenge_message(secmech: &Properties, challenge: &str) -> String {
    let name = secmech.get("name").map(String::as_str).unwrap_or("null");
    let inputinfo = secmech
        .get("inputinfo")
        .map(String::as_str)
        .unwrap_or("null");

    format!("{name}\n{inputinfo}\n\n{challenge}")
}

async fn request_pin(
    passport: &mut PinTanPassport,
    callback: Option<&dyn HbciCallback>,
) -> HbciResult<String> {
    if let Some(pin) = passport.pin() {
        return Ok(pin.to_owned());
    }
    let Some(callback) = callback else {
        return Err(HbciError::new(
            HbciErrorKind::Callback,
            "callback required for PIN",
        ));
    };

    let response = callback
        .handle(CallbackEvent {
            reason: CallbackReason::NeedPtPin,
            message: "Please enter your PIN for PIN/TAN now".to_owned(),
            data_type: CallbackDataType::Secret,
            current_value: None,
        })
        .await?;
    let Some(pin) = response.value.filter(|value| !value.is_empty()) else {
        return Err(HbciError::new(
            HbciErrorKind::Callback,
            "PIN must not be of length zero",
        ));
    };

    passport.set_pin(pin.clone());
    Ok(pin)
}

async fn apply_pintan_signature(
    message: &mut HbciMessage,
    sig_head_path: &str,
    sig_tail_path: &str,
    passport: &mut PinTanPassport,
    callback: Option<&dyn HbciCallback>,
) -> HbciResult<()> {
    let signature_context = PinTanSignatureContext::generate()?;
    let sig_head = signature_context.sig_head_from_passport(passport)?;
    apply_pintan_sig_head(message, sig_head_path, &sig_head)?;
    apply_pintan_sig_tail_from_head(message, sig_head_path, sig_tail_path)?;
    message.prepare_outgoing()?;
    let signed_range = collect_pintan_signature_range(message, sig_head_path, sig_tail_path)?;
    let signature = sign_pintan_user_sig(passport, callback, Some(&signed_range)).await?;
    apply_pintan_user_sig_to_sig_tail(message, sig_tail_path, &signature)
}

async fn sign_pintan_user_sig_for_sca(
    passport: &mut PinTanPassport,
    callback: Option<&dyn HbciCallback>,
) -> HbciResult<Vec<u8>> {
    sign_pintan_user_sig(passport, callback, None).await
}

async fn sign_pintan_user_sig(
    passport: &mut PinTanPassport,
    callback: Option<&dyn HbciCallback>,
    signed_range: Option<&str>,
) -> HbciResult<Vec<u8>> {
    let pin = request_pin(passport, callback).await?;
    let tan = if passport
        .current_tan_method()
        .unwrap_or(ONESTEP_TAN_METHOD_ID)
        == ONESTEP_TAN_METHOD_ID
    {
        request_tan_for_one_step(passport, callback, signed_range).await?
    } else {
        request_tan_for_sca(passport, callback).await?
    };
    UserSig::encode(Some(&pin), tan.as_deref())
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
        "AccInfo" => render_acc_info(message, job, index, passport),
        "CardList" => render_card_list(message, job, index, passport),
        "ChangePIN" => render_change_pin(message, job, index),
        "DauerDel" => render_dauer_del(message, job, index, passport),
        "DauerEdit" => render_dauer_edit(message, job, index, passport),
        "DauerList" => render_dauer_list(message, job, index, passport),
        "DauerNew" => render_dauer_new(message, job, index, passport),
        "DauerSEPADel" => render_dauer_sepa_del(message, job, index, passport),
        "DauerSEPAEdit" => render_dauer_sepa_edit(message, job, index, passport),
        "DauerSEPAList" => render_dauer_sepa_list(message, job, index, passport),
        "DauerSEPANew" => render_dauer_sepa_new(message, job, index, passport),
        "DauerLastSEPAList" => render_dauer_last_sepa_list(message, job, index, passport),
        "DauerLastSEPANew" => render_dauer_last_sepa_new(message, job, index, passport),
        "FestCondList" => render_fest_cond_list(message, job, index),
        "FestList" => render_fest_list(message, job, index, passport),
        "InfoList" => render_info_list(message, job, index),
        "InfoOrder" => render_info_order(message, job, index),
        "InstUebSEPA" => render_inst_ueb_sepa(message, job, index, passport),
        "Kontoauszug" => render_kontoauszug(message, job, index, passport),
        "KontoauszugPdf" => render_kontoauszug_pdf(message, job, index, passport),
        "KUmsAll" => render_kums_all(message, job, index, passport),
        "KUmsAllCamt" => render_kums_all_camt(message, job, index, passport),
        "KUmsNew" => render_kums_new(message, job, index, passport),
        "KUmsZeitSEPA" => render_kums_zeit_sepa(message, job, index, passport),
        "LastB2BSEPA" => render_last_b2b_sepa(message, job, index, passport),
        "LastCOR1SEPA" => render_last_cor1_sepa(message, job, index, passport),
        "LastSEPA" => render_last_sepa(message, job, index, passport),
        "MultiLastB2BSEPA" => render_multi_last_b2b_sepa(message, job, index, passport),
        "MultiLastCOR1SEPA" => render_multi_last_cor1_sepa(message, job, index, passport),
        "MultiLastSEPA" => render_multi_last_sepa(message, job, index, passport),
        "MultiUebSEPA" => render_multi_ueb_sepa(message, job, index, passport),
        "Receipt" => render_receipt(message, job, index),
        "SEPAInfo" => render_sepa_info(message, index),
        "SaldoReq" => render_saldo_request(message, job, index, passport),
        "SaldoReqAll" => render_saldo_request_all(message, job, index, passport),
        "Status" => render_status(message, job, index),
        "TANList" => render_tan_list(message, index),
        "TANMediaList" => render_tan_media_list(message, job, index),
        "TAN2Step" => render_tan2step(message, job, index),
        "TermUeb" => render_term_ueb(message, job, index, passport),
        "TermUebDel" => render_term_ueb_del(message, job, index, passport),
        "TermUebEdit" => render_term_ueb_edit(message, job, index, passport),
        "TermUebList" => render_term_ueb_list(message, job, index, passport),
        "TermMultiUebSEPA" => render_term_multi_ueb_sepa(message, job, index, passport),
        "TermUebSEPA" => render_term_ueb_sepa(message, job, index, passport),
        "TermUebSEPADel" => render_term_ueb_sepa_del(message, job, index, passport),
        "TermUebSEPAEdit" => render_term_ueb_sepa_edit(message, job, index, passport),
        "TermUebSEPAList" => render_term_ueb_sepa_list(message, job, index, passport),
        "Ueb" => render_ueb(message, job, index, passport),
        "UebBZU" => render_ueb_bzu(message, job, index, passport),
        "UebEil" => render_ueb_eil(message, job, index, passport),
        "UebForeign" => render_ueb_foreign(message, job, index, passport),
        "UebSEPA" => render_ueb_sepa(message, job, index, passport),
        "Umb" => render_umb(message, job, index, passport),
        "UmbSEPA" => render_umb_sepa(message, job, index, passport),
        "VoP" => render_vop(message, job, index),
        "VoPAuth" => render_vop_auth(message, job, index),
        "WPDepotList" => render_wp_depot_list(message, job, index, passport),
        "WPDepotUms" => render_wp_depot_ums(message, job, index, passport),
        name => Err(HbciError::new(
            HbciErrorKind::Unsupported,
            format!("queued job rendering is not ported yet for {name}"),
        )),
    }
}

fn new_tan2step_process1_job(
    registry: &JobRegistry,
    hbci_version: &str,
    passport: &PinTanPassport,
    task: &HbciJob,
    tan_media: Option<&str>,
    challenge_info: Option<&ChallengeInfo>,
) -> HbciResult<HbciJob> {
    let task_info = orderhash_source_job_info(task.name())?;
    let task_segment = render_task_segment_for_orderhash(hbci_version, task, passport)?;
    let order_hash_mode_code = passport.order_hash_mode_code().ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::InvalidArgument,
            "PinTAN BPD does not contain orderhashmode for current HKTAN segment version",
        )
    })?;
    let order_hash =
        OrderHashMode::from_code(&order_hash_mode_code)?.hash_segment(&task_segment)?;
    let secmech = passport.current_secmech_info();

    let mut hktan = registry.new_job("TAN2Step")?;
    hktan.try_set_param("process", "1")?;
    hktan.try_set_param("ordersegcode", task_info.code)?;
    hktan.try_set_param("notlasttan", "N")?;
    hktan.try_set_param("orderhash", order_hash)?;

    if passport.tan2step_parameter("needorderaccount").as_deref() == Some("2")
        && let Some(account) = task_order_account(task, passport)
    {
        hktan.set_param_account("orderaccount", &account);
    }

    if let Some(tan_media) = tan_media.filter(|value| !value.is_empty()) {
        hktan.try_set_param("tanmedia", tan_media)?;
    }

    apply_challenge_params_if_needed(&mut hktan, task_info.code, task, &secmech, challenge_info)?;

    Ok(hktan)
}

fn new_tan2step_initial_job(
    registry: &JobRegistry,
    hbci_version: &str,
    passport: &PinTanPassport,
    task: &HbciJob,
    tan_media: Option<&str>,
    challenge_info: Option<&ChallengeInfo>,
) -> HbciResult<HbciJob> {
    if passport.tan2step_parameter("process").as_deref() == Some("1") {
        return new_tan2step_process1_job(
            registry,
            hbci_version,
            passport,
            task,
            tan_media,
            challenge_info,
        );
    }

    new_tan2step_process2_step1_job(registry, task, tan_media)
}

fn new_tan2step_process2_step1_job(
    registry: &JobRegistry,
    task: &HbciJob,
    tan_media: Option<&str>,
) -> HbciResult<HbciJob> {
    let task_info = orderhash_source_job_info(task.name())?;
    let mut hktan = registry.new_job("TAN2Step")?;
    hktan.try_set_param("process", "4")?;
    hktan.try_set_param("ordersegcode", task_info.code)?;

    if let Some(tan_media) = tan_media.filter(|value| !value.is_empty()) {
        hktan.try_set_param("tanmedia", tan_media)?;
    }

    Ok(hktan)
}

fn new_tan2step_process2_job(
    registry: &JobRegistry,
    passport: &PinTanPassport,
) -> HbciResult<HbciJob> {
    let order_ref = passport
        .sca_state()
        .order_ref
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                "PinTAN SCA state does not contain an order reference for process-2 HKTAN",
            )
        })?;

    let mut hktan = registry.new_job("TAN2Step")?;
    hktan.try_set_param("process", "2")?;
    hktan.try_set_param("orderref", order_ref)?;
    hktan.try_set_param("notlasttan", "N")?;

    Ok(hktan)
}

fn render_task_segment_for_orderhash(
    hbci_version: &str,
    task: &HbciJob,
    passport: &PinTanPassport,
) -> HbciResult<String> {
    let syntax = load_protocol_spec(hbci_version)?.parse_syntax()?;
    let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg")?;
    let task_info = orderhash_source_job_info(task.name())?;

    render_job_into_custom_message(&mut message, task, 0, passport)?;
    message.set_value(&format!("{}.SegHead.seq", task_info.path), "3")?;
    message
        .element(task_info.path)
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::Protocol,
                format!("message element path {} is not defined", task_info.path),
            )
        })?
        .to_fints_string()
}

fn task_order_account(task: &HbciJob, passport: &PinTanPassport) -> Option<Konto> {
    let task_info = orderhash_source_job_info(task.name()).ok()?;
    let account = match task.name() {
        "Kontoauszug" | "KontoauszugPdf" => {
            effective_job_my_account(task, passport, task_info.lowlevel_segment, "my")
        }
        "WPDepotList" => wp_depot_list_account(task, passport).ok()?,
        "WPDepotUms" => wp_depot_ums_account(task, passport).ok()?,
        _ => effective_job_account(task, passport, task_info.lowlevel_segment, "my"),
    };
    has_account_identity(&account).then_some(account)
}

fn apply_challenge_params_if_needed(
    hktan: &mut HbciJob,
    task_code: &str,
    task: &HbciJob,
    secmech: &Properties,
    challenge_info: Option<&ChallengeInfo>,
) -> HbciResult<()> {
    if secmech.get("needchallengeklass").map(String::as_str) != Some("J") {
        return Ok(());
    }

    let Some(challenge_info) = challenge_info else {
        return Ok(());
    };
    let Some(applied) = challenge_info.apply_params(task_code, task.lowlevel_params(), secmech)?
    else {
        return Ok(());
    };

    for (name, value) in applied.to_hktan_params() {
        hktan.try_set_param(name, value)?;
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct OrderhashSourceJobInfo {
    code: &'static str,
    lowlevel_segment: &'static str,
    path: &'static str,
}

fn orderhash_source_job_info(job_name: &str) -> HbciResult<OrderhashSourceJobInfo> {
    match job_name {
        "AccInfo" => Ok(OrderhashSourceJobInfo {
            code: "HKKIF",
            lowlevel_segment: "AccInfo2",
            path: "CustomMsg.GV.AccInfo2",
        }),
        "CardList" => Ok(OrderhashSourceJobInfo {
            code: "HKAZK",
            lowlevel_segment: "CardList2",
            path: "CustomMsg.GV.CardList2",
        }),
        "ChangePIN" => Ok(OrderhashSourceJobInfo {
            code: "HKPAE",
            lowlevel_segment: "ChangePIN1",
            path: "CustomMsg.GV.ChangePIN1",
        }),
        "DauerDel" => Ok(OrderhashSourceJobInfo {
            code: "HKDAL",
            lowlevel_segment: "DauerDel4",
            path: "CustomMsg.GV.DauerDel4",
        }),
        "DauerEdit" => Ok(OrderhashSourceJobInfo {
            code: "HKDAN",
            lowlevel_segment: "DauerEdit5",
            path: "CustomMsg.GV.DauerEdit5",
        }),
        "DauerList" => Ok(OrderhashSourceJobInfo {
            code: "HKDAB",
            lowlevel_segment: "DauerList5",
            path: "CustomMsg.GV.DauerList5",
        }),
        "DauerNew" => Ok(OrderhashSourceJobInfo {
            code: "HKDAE",
            lowlevel_segment: "DauerNew5",
            path: "CustomMsg.GV.DauerNew5",
        }),
        "DauerSEPAList" => Ok(OrderhashSourceJobInfo {
            code: "HKCDB",
            lowlevel_segment: "DauerSEPAList2",
            path: "CustomMsg.GV.DauerSEPAList2",
        }),
        "DauerSEPANew" => Ok(OrderhashSourceJobInfo {
            code: "HKCDE",
            lowlevel_segment: "DauerSEPANew1",
            path: "CustomMsg.GV.DauerSEPANew1",
        }),
        "DauerLastSEPANew" => Ok(OrderhashSourceJobInfo {
            code: "HKDDE",
            lowlevel_segment: "DauerLastSEPANew1",
            path: "CustomMsg.GV.DauerLastSEPANew1",
        }),
        "DauerLastSEPAList" => Ok(OrderhashSourceJobInfo {
            code: "HKDDB",
            lowlevel_segment: "DauerLastSEPAList1",
            path: "CustomMsg.GV.DauerLastSEPAList1",
        }),
        "DauerSEPAEdit" => Ok(OrderhashSourceJobInfo {
            code: "HKCDN",
            lowlevel_segment: "DauerSEPAEdit1",
            path: "CustomMsg.GV.DauerSEPAEdit1",
        }),
        "DauerSEPADel" => Ok(OrderhashSourceJobInfo {
            code: "HKCDL",
            lowlevel_segment: "DauerSEPADel1",
            path: "CustomMsg.GV.DauerSEPADel1",
        }),
        "TermUebList" => Ok(OrderhashSourceJobInfo {
            code: "HKTUB",
            lowlevel_segment: "TermUebList3",
            path: "CustomMsg.GV.TermUebList3",
        }),
        "TermUeb" => Ok(OrderhashSourceJobInfo {
            code: "HKTUE",
            lowlevel_segment: "TermUeb4",
            path: "CustomMsg.GV.TermUeb4",
        }),
        "TermUebDel" => Ok(OrderhashSourceJobInfo {
            code: "HKTUL",
            lowlevel_segment: "TermUebDel3",
            path: "CustomMsg.GV.TermUebDel3",
        }),
        "TermUebEdit" => Ok(OrderhashSourceJobInfo {
            code: "HKTUA",
            lowlevel_segment: "TermUebEdit4",
            path: "CustomMsg.GV.TermUebEdit4",
        }),
        "TermUebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKCSE",
            lowlevel_segment: "TermUebSEPA1",
            path: "CustomMsg.GV.TermUebSEPA1",
        }),
        "TermUebSEPADel" => Ok(OrderhashSourceJobInfo {
            code: "HKCSL",
            lowlevel_segment: "TermUebSEPADel1",
            path: "CustomMsg.GV.TermUebSEPADel1",
        }),
        "TermUebSEPAEdit" => Ok(OrderhashSourceJobInfo {
            code: "HKCSA",
            lowlevel_segment: "TermUebSEPAEdit1",
            path: "CustomMsg.GV.TermUebSEPAEdit1",
        }),
        "TermUebSEPAList" => Ok(OrderhashSourceJobInfo {
            code: "HKCSB",
            lowlevel_segment: "TermUebSEPAList1",
            path: "CustomMsg.GV.TermUebSEPAList1",
        }),
        "TermMultiUebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKCME",
            lowlevel_segment: "TermSammelUebSEPA1",
            path: "CustomMsg.GV.TermSammelUebSEPA1",
        }),
        "InstUebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKIPZ",
            lowlevel_segment: "InstUebSEPA1",
            path: "CustomMsg.GV.InstUebSEPA1",
        }),
        "LastB2BSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKBSE",
            lowlevel_segment: "LastB2BSEPA1",
            path: "CustomMsg.GV.LastB2BSEPA1",
        }),
        "LastCOR1SEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKDSC",
            lowlevel_segment: "LastCOR1SEPA1",
            path: "CustomMsg.GV.LastCOR1SEPA1",
        }),
        "LastSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKDSE",
            lowlevel_segment: "LastSEPA1",
            path: "CustomMsg.GV.LastSEPA1",
        }),
        "MultiLastSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKDME",
            lowlevel_segment: "SammelLastSEPA1",
            path: "CustomMsg.GV.SammelLastSEPA1",
        }),
        "MultiLastCOR1SEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKDMC",
            lowlevel_segment: "SammelLastCOR1SEPA1",
            path: "CustomMsg.GV.SammelLastCOR1SEPA1",
        }),
        "MultiLastB2BSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKBME",
            lowlevel_segment: "SammelLastB2BSEPA1",
            path: "CustomMsg.GV.SammelLastB2BSEPA1",
        }),
        "Ueb" => Ok(OrderhashSourceJobInfo {
            code: "HKUEB",
            lowlevel_segment: "Ueb5",
            path: "CustomMsg.GV.Ueb5",
        }),
        "UebBZU" => Ok(OrderhashSourceJobInfo {
            code: "HKUEB",
            lowlevel_segment: "Ueb5",
            path: "CustomMsg.GV.Ueb5",
        }),
        "UebEil" => Ok(OrderhashSourceJobInfo {
            code: "HKEIL",
            lowlevel_segment: "UebEil1",
            path: "CustomMsg.GV.UebEil1",
        }),
        "UebForeign" => Ok(OrderhashSourceJobInfo {
            code: "HKAOM",
            lowlevel_segment: "UebForeign2",
            path: "CustomMsg.GV.UebForeign2",
        }),
        "Umb" => Ok(OrderhashSourceJobInfo {
            code: "HKUMB",
            lowlevel_segment: "Umb2",
            path: "CustomMsg.GV.Umb2",
        }),
        "InfoList" => Ok(OrderhashSourceJobInfo {
            code: "HKKIA",
            lowlevel_segment: "InfoList4",
            path: "CustomMsg.GV.InfoList4",
        }),
        "InfoOrder" => Ok(OrderhashSourceJobInfo {
            code: "HKINF",
            lowlevel_segment: "InfoDetails4",
            path: "CustomMsg.GV.InfoDetails4",
        }),
        "UebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKCCS",
            lowlevel_segment: "UebSEPA1",
            path: "CustomMsg.GV.UebSEPA1",
        }),
        "MultiUebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKCCM",
            lowlevel_segment: "SammelUebSEPA1",
            path: "CustomMsg.GV.SammelUebSEPA1",
        }),
        "UmbSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKCUM",
            lowlevel_segment: "UmbSEPA1",
            path: "CustomMsg.GV.UmbSEPA1",
        }),
        "KUmsAll" => Ok(OrderhashSourceJobInfo {
            code: "HKKAZ",
            lowlevel_segment: "KUmsZeit7",
            path: "CustomMsg.GV.KUmsZeit7",
        }),
        "KUmsZeitSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKKAZ",
            lowlevel_segment: "KUmsZeitSEPA7",
            path: "CustomMsg.GV.KUmsZeitSEPA7",
        }),
        "KUmsAllCamt" => Ok(OrderhashSourceJobInfo {
            code: "HKCAZ",
            lowlevel_segment: "KUmsZeitCamt1",
            path: "CustomMsg.GV.KUmsZeitCamt1",
        }),
        "KUmsNew" => Ok(OrderhashSourceJobInfo {
            code: "HKKAN",
            lowlevel_segment: "KUmsNew7",
            path: "CustomMsg.GV.KUmsNew7",
        }),
        "Receipt" => Ok(OrderhashSourceJobInfo {
            code: "HKQTG",
            lowlevel_segment: "Receipt1",
            path: "CustomMsg.GV.Receipt1",
        }),
        "SEPAInfo" => Ok(OrderhashSourceJobInfo {
            code: "HKSPA",
            lowlevel_segment: "SEPAInfo1",
            path: "CustomMsg.GV.SEPAInfo1",
        }),
        "Status" => Ok(OrderhashSourceJobInfo {
            code: "HKPRO",
            lowlevel_segment: "Status4",
            path: "CustomMsg.GV.Status4",
        }),
        "TANList" => Ok(OrderhashSourceJobInfo {
            code: "HKTAZ",
            lowlevel_segment: "TANListList1",
            path: "CustomMsg.GV.TANListList1",
        }),
        "FestCondList" => Ok(OrderhashSourceJobInfo {
            code: "HKFGK",
            lowlevel_segment: "FestCondList3",
            path: "CustomMsg.GV.FestCondList3",
        }),
        "FestList" => Ok(OrderhashSourceJobInfo {
            code: "HKFGB",
            lowlevel_segment: "FestList4",
            path: "CustomMsg.GV.FestList4",
        }),
        "Kontoauszug" => Ok(OrderhashSourceJobInfo {
            code: "HKEKA",
            lowlevel_segment: "Kontoauszug5",
            path: "CustomMsg.GV.Kontoauszug5",
        }),
        "KontoauszugPdf" => Ok(OrderhashSourceJobInfo {
            code: "HKEKP",
            lowlevel_segment: "KontoauszugPdf2",
            path: "CustomMsg.GV.KontoauszugPdf2",
        }),
        "VoP" => Ok(OrderhashSourceJobInfo {
            code: "HKVPP",
            lowlevel_segment: "VoPCheck1",
            path: "CustomMsg.GV.VoPCheck1",
        }),
        "VoPAuth" => Ok(OrderhashSourceJobInfo {
            code: "HKVPA",
            lowlevel_segment: "VoPAuth1",
            path: "CustomMsg.GV.VoPAuth1",
        }),
        "WPDepotList" => Ok(OrderhashSourceJobInfo {
            code: "HKWPD",
            lowlevel_segment: "WPDepotList6",
            path: "CustomMsg.GV.WPDepotList6",
        }),
        "WPDepotUms" => Ok(OrderhashSourceJobInfo {
            code: "HKWDU",
            lowlevel_segment: "WPDepotUms5",
            path: "CustomMsg.GV.WPDepotUms5",
        }),
        "SaldoReq" | "SaldoReqAll" => Ok(OrderhashSourceJobInfo {
            code: "HKSAL",
            lowlevel_segment: "Saldo7",
            path: "CustomMsg.GV.Saldo7",
        }),
        name => Err(HbciError::new(
            HbciErrorKind::Unsupported,
            format!("process-1 HKTAN preparation is not ported yet for {name}"),
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

fn acc_info_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.AccInfoRes2".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.AccInfoRes2", index + 1)
    }
}

fn card_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.CardListRes2".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.CardListRes2", index + 1)
    }
}

fn info_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.InfoListRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.InfoListRes4", index + 1)
    }
}

fn info_order_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.InfoDetailsRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.InfoDetailsRes4", index + 1)
    }
}

fn fest_cond_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.FestCondListRes3".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.FestCondListRes3", index + 1)
    }
}

fn fest_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.FestListRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.FestListRes4", index + 1)
    }
}

fn kontoauszug_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.KontoauszugRes5".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.KontoauszugRes5", index + 1)
    }
}

fn kontoauszug_pdf_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.KontoauszugPdfRes2".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.KontoauszugPdfRes2", index + 1)
    }
}

fn dauer_sepa_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerSEPAListRes2".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerSEPAListRes2", index + 1)
    }
}

fn dauer_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerListRes5".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerListRes5", index + 1)
    }
}

fn dauer_edit_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerEditRes5".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerEditRes5", index + 1)
    }
}

fn dauer_sepa_edit_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerSEPAEditRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerSEPAEditRes1", index + 1)
    }
}

fn dauer_new_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerNewRes5".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerNewRes5", index + 1)
    }
}

fn dauer_sepa_new_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerSEPANewRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerSEPANewRes1", index + 1)
    }
}

fn dauer_last_sepa_new_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerLastSEPANewRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerLastSEPANewRes1", index + 1)
    }
}

fn dauer_last_sepa_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerLastSEPAListRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerLastSEPAListRes1", index + 1)
    }
}

fn tan_media_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TANMediaListRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TANMediaListRes4", index + 1)
    }
}

fn term_ueb_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermUebSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermUebSEPARes1", index + 1)
    }
}

fn term_ueb_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermUebRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermUebRes4", index + 1)
    }
}

fn term_multi_ueb_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermSammelUebSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermSammelUebSEPARes1", index + 1)
    }
}

fn term_ueb_edit_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermUebEditRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermUebEditRes4", index + 1)
    }
}

fn inst_ueb_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.InstUebSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.InstUebSEPARes1", index + 1)
    }
}

fn last_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.LastSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.LastSEPARes1", index + 1)
    }
}

fn last_cor1_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.LastCOR1SEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.LastCOR1SEPARes1", index + 1)
    }
}

fn last_b2b_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.LastB2BSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.LastB2BSEPARes1", index + 1)
    }
}

fn multi_last_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.SammelLastSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.SammelLastSEPARes1", index + 1)
    }
}

fn multi_last_cor1_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.SammelLastCOR1SEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.SammelLastCOR1SEPARes1", index + 1)
    }
}

fn multi_last_b2b_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.SammelLastB2BSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.SammelLastB2BSEPARes1", index + 1)
    }
}

fn term_ueb_sepa_edit_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermUebSEPAEditRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermUebSEPAEditRes1", index + 1)
    }
}

fn term_ueb_sepa_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermUebSEPAListRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermUebSEPAListRes1", index + 1)
    }
}

fn term_ueb_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.TermUebListRes3".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.TermUebListRes3", index + 1)
    }
}

fn sepa_info_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.SEPAInfoRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.SEPAInfoRes1", index + 1)
    }
}

fn vop_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.VoPCheckRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.VoPCheckRes1", index + 1)
    }
}

fn wp_depot_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.WPDepotListRes6".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.WPDepotListRes6", index + 1)
    }
}

fn wp_depot_ums_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.WPDepotUmsRes5".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.WPDepotUmsRes5", index + 1)
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

fn render_card_list(
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
    let segment = format!("{root}.CardList2");
    let account = effective_job_account(job, passport, "CardList2", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "CardList requires my.number or a passport account for the current CardList2 renderer",
        ));
    }

    set_national_account_values(message, &segment, &account)?;

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

fn render_kums_zeit_sepa(
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
    let segment = format!("{root}.KUmsZeitSEPA7");
    let account = effective_job_account(job, passport, "KUmsZeitSEPA7", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "KUmsZeitSEPA requires my.iban or a passport account for the current KUmsZeitSEPA7 tracer renderer",
        ));
    }

    set_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "KUmsZeitSEPA7.allaccounts", "all").unwrap_or("N"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "KUmsZeitSEPA7.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "KUmsZeitSEPA7.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "KUmsZeitSEPA7.maxentries", "maxentries"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.offset"),
        job_param(job, "KUmsZeitSEPA7.offset", "offset"),
    )?;

    Ok(())
}

fn render_fest_cond_list(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.FestCondList3");

    message.set_value(
        &format!("{segment}.curr"),
        job_param(job, "FestCondList3.curr", "curr").unwrap_or("EUR"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "FestCondList3.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_fest_list(
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
    let segment = format!("{root}.FestList4");
    let account = effective_job_account(job, passport, "FestList4", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "FestList requires my.number or a passport account for the current FestList4 renderer",
        ));
    }

    set_national_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "FestList4.allaccounts", "dummy").unwrap_or("N"),
    )?;

    Ok(())
}

fn render_kontoauszug(
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
    let segment = format!("{root}.Kontoauszug5");
    let account = effective_job_my_account(job, passport, "Kontoauszug5", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "Kontoauszug requires my.iban, my.number, or a passport account for the current Kontoauszug5 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    set_optional_message_value(
        message,
        &format!("{segment}.format"),
        job_param(job, "Kontoauszug5.format", "format"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.idx"),
        job_param(job, "Kontoauszug5.idx", "idx"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.year"),
        job_param(job, "Kontoauszug5.year", "year"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "Kontoauszug5.maxentries", "maxentries"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.offset"),
        job_param(job, "Kontoauszug5.offset", "offset"),
    )?;

    Ok(())
}

fn render_kontoauszug_pdf(
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
    let segment = format!("{root}.KontoauszugPdf2");
    let account = effective_job_my_account(job, passport, "KontoauszugPdf2", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "KontoauszugPdf requires my.iban, my.number, or a passport account for the current KontoauszugPdf2 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    set_optional_message_value(
        message,
        &format!("{segment}.idx"),
        job_param(job, "KontoauszugPdf2.idx", "idx"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.year"),
        job_param(job, "KontoauszugPdf2.year", "year"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "KontoauszugPdf2.maxentries", "maxentries"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.offset"),
        job_param(job, "KontoauszugPdf2.offset", "offset"),
    )?;

    Ok(())
}

fn render_info_list(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.InfoList4");

    message.set_value(&segment, "requested")?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "InfoList4.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_info_order(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.InfoDetails4");

    message.set_value(&segment, "requested")?;
    message.set_value(
        &format!("{segment}.InfoCodes.code"),
        job_param_required(
            job,
            "InfoDetails4.InfoCodes.code",
            "code",
            "InfoOrder requires code",
        )?,
    )?;
    for index in 2..=10 {
        set_optional_message_value(
            message,
            &format!("{segment}.InfoCodes.code_{index}"),
            job_param(
                job,
                &format!("InfoDetails4.InfoCodes.code_{index}"),
                &format!("code_{index}"),
            ),
        )?;
    }
    for (suffix, frontend) in [
        ("name1", "name"),
        ("name2", "name2"),
        ("street_pf", "street"),
        ("ort", "ort"),
        ("plz_ort", "plz"),
        ("plz", "plz"),
        ("country", "country"),
        ("tel", "tel"),
        ("fax", "fax"),
        ("email", "email"),
    ] {
        let path = format!("{segment}.Address.{suffix}");
        if message.element(&path).is_none() {
            continue;
        }
        set_optional_message_value(
            message,
            &path,
            job_param(job, &format!("InfoDetails4.Address.{suffix}"), frontend),
        )?;
    }

    Ok(())
}

fn render_status(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.Status4");

    message.set_value(&segment, "requested")?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "Status4.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "Status4.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "Status4.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_tan_list(message: &mut HbciMessage, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    message.set_value(&format!("{root}.TANListList1"), "requested")
}

fn render_change_pin(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.ChangePIN1");

    message.set_value(&segment, "requested")?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.newpin"),
        job,
        "ChangePIN1.newpin",
        "newpin",
        "ChangePIN requires newpin",
    )
}

fn render_vop(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.VoPCheck1");
    let descriptor = job
        .lowlevel_param("VoPCheck1.suppreports.descriptor")
        .or_else(|| job.param("suppreports.descriptor"))
        .unwrap_or("");
    let polling_id = job_param_required(
        job,
        "VoPCheck1.pollingid",
        "pollingid",
        "VoP requires pollingid",
    )?;
    let max_entries = job_param_required(
        job,
        "VoPCheck1.maxentries",
        "maxentries",
        "VoP requires maxentries",
    )?;
    let offset = job_param_required(job, "VoPCheck1.offset", "offset", "VoP requires offset")?;

    message.set_value(&segment, "requested")?;
    message.set_value(&format!("{segment}.suppreports.descriptor"), descriptor)?;
    message.set_value(
        &format!("{segment}.pollingid"),
        sepa_binary_value(polling_id),
    )?;
    message.set_value(&format!("{segment}.maxentries"), max_entries)?;
    message.set_value(&format!("{segment}.offset"), offset)
}

fn render_vop_auth(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.VoPAuth1");
    let vopid = job_param_required(job, "VoPAuth1.vopid", "vopid", "VoPAuth requires vopid")?;

    message.set_value(&segment, "requested")?;
    message.set_value(&format!("{segment}.vopid"), sepa_binary_value(vopid))
}

fn render_wp_depot_list(
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
    let segment = format!("{root}.WPDepotList6");
    let account = wp_depot_list_account(job, passport)?;

    message.set_value(&segment, "requested")?;
    set_classic_national_account_values(message, &format!("{segment}.Depot"), &account)?;
    set_optional_message_value(
        message,
        &format!("{segment}.quality"),
        job_param(job, "WPDepotList6.quality", "quality"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "WPDepotList6.maxentries", "maxentries"),
    )
}

fn render_wp_depot_ums(
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
    let segment = format!("{root}.WPDepotUms5");
    let account = wp_depot_ums_account(job, passport)?;

    message.set_value(&segment, "requested")?;
    set_classic_national_account_values(message, &format!("{segment}.Depot"), &account)?;
    message.set_value(
        &format!("{segment}.alldepots"),
        job_param(job, "WPDepotUms5.alldepots", "dummy").unwrap_or("N"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "WPDepotUms5.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "WPDepotUms5.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "WPDepotUms5.maxentries", "maxentries"),
    )
}

fn render_acc_info(
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
    let segment = format!("{root}.AccInfo2");
    let account = effective_job_account(job, passport, "AccInfo2", "my");
    if account.number.as_deref().is_none_or(str::is_empty) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "AccInfo requires my.number or a passport account for the current AccInfo2 tracer renderer",
        ));
    }

    set_national_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "AccInfo2.allaccounts", "all").unwrap_or("N"),
    )?;

    Ok(())
}

fn render_dauer_sepa_list(
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
    let segment = format!("{root}.DauerSEPAList2");
    let account = dauer_sepa_list_account(job, passport);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerSEPAList requires my.iban, src.iban, my.number, or a passport account for the current DauerSEPAList2 tracer renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "DauerSEPAList2.sepadescr", "_sepadescriptor")
            .unwrap_or(PAIN_001_001_02_URN),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.orderid"),
        job_param(job, "DauerSEPAList2.orderid", "orderid"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "DauerSEPAList2.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_dauer_last_sepa_list(
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
    let segment = format!("{root}.DauerLastSEPAList1");
    let account = effective_job_my_account(job, passport, "DauerLastSEPAList1", "src");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerLastSEPAList requires src.iban, src.number, or a passport account for the current DauerLastSEPAList1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "DauerLastSEPAList1.sepadescr", "_sepadescriptor")
            .unwrap_or(PAIN_008_001_02_URN),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.orderid"),
        job_param(job, "DauerLastSEPAList1.orderid", "orderid"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "DauerLastSEPAList1.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_dauer_list(
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
    let segment = format!("{root}.DauerList5");
    let account = effective_job_account(job, passport, "DauerList5", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerList requires my.number or a passport account for the current DauerList5 renderer",
        ));
    }

    set_national_account_values(message, &segment, &account)?;
    set_optional_message_value(
        message,
        &format!("{segment}.orderid"),
        job_param(job, "DauerList5.orderid", "orderid"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "DauerList5.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_term_ueb_sepa_list(
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
    let segment = format!("{root}.TermUebSEPAList1");
    let account = term_ueb_sepa_list_account(job, passport);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebSEPAList requires my.iban, src.iban, my.number, or a passport account for the current TermUebSEPAList1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "TermUebSEPAList1.sepadescr", "_sepadescriptor")
            .unwrap_or(PAIN_001_001_02_URN),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "TermUebSEPAList1.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "TermUebSEPAList1.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "TermUebSEPAList1.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_term_ueb_list(
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
    let segment = format!("{root}.TermUebList3");
    let account = effective_job_account(job, passport, "TermUebList3", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebList requires my.number or a passport account for the current TermUebList3 renderer",
        ));
    }

    set_national_account_values(message, &segment, &account)?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "TermUebList3.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "TermUebList3.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "TermUebList3.maxentries", "maxentries"),
    )?;

    Ok(())
}

fn render_dauer_new(
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
    let lowlevel_segment = "DauerNew5";
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerNew requires src.number or a passport account for the current DauerNew5 renderer",
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    if !has_account_identity(&dst_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerNew requires dst.number for the current DauerNew5 renderer",
        ));
    }

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_classic_national_account_values(message, &format!("{segment}.Other"), &dst_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.name"),
        job,
        "DauerNew5.name",
        "name",
        "DauerNew requires name",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.name2"),
        job_param(job, "DauerNew5.name2", "name2"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        "DauerNew5.BTG.value",
        "btg.value",
        "DauerNew requires btg.value",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        "DauerNew5.BTG.curr",
        "btg.curr",
        "DauerNew requires btg.curr",
    )?;
    message.set_value(
        &format!("{segment}.key"),
        job_param(job, "DauerNew5.key", "key").unwrap_or("52"),
    )?;
    for usage_index in 0..CLASSIC_USAGE_LINE_COUNT {
        let usage_name = classic_usage_frontend_name(usage_index);
        set_optional_message_value(
            message,
            &format!("{segment}.usage.{usage_name}"),
            job_param(job, &format!("DauerNew5.usage.{usage_name}"), &usage_name),
        )?;
    }
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.firstdate"),
        job,
        "DauerNew5.DauerDetails.firstdate",
        "firstdate",
        "DauerNew requires firstdate",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.timeunit"),
        job,
        "DauerNew5.DauerDetails.timeunit",
        "timeunit",
        "DauerNew requires timeunit",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.turnus"),
        job,
        "DauerNew5.DauerDetails.turnus",
        "turnus",
        "DauerNew requires turnus",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.execday"),
        job,
        "DauerNew5.DauerDetails.execday",
        "execday",
        "DauerNew requires execday",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.DauerDetails.lastdate"),
        job_param(job, "DauerNew5.DauerDetails.lastdate", "lastdate"),
    )?;

    Ok(())
}

fn render_dauer_edit(
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
    let lowlevel_segment = "DauerEdit5";
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerEdit requires src.number or a passport account for the current DauerEdit5 renderer",
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    if !has_account_identity(&dst_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerEdit requires dst.number for the current DauerEdit5 renderer",
        ));
    }

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_classic_national_account_values(message, &format!("{segment}.Other"), &dst_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.name"),
        job,
        "DauerEdit5.name",
        "name",
        "DauerEdit requires name",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.name2"),
        job_param(job, "DauerEdit5.name2", "name2"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        "DauerEdit5.BTG.value",
        "btg.value",
        "DauerEdit requires btg.value",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        "DauerEdit5.BTG.curr",
        "btg.curr",
        "DauerEdit requires btg.curr",
    )?;
    message.set_value(
        &format!("{segment}.key"),
        job_param(job, "DauerEdit5.key", "key").unwrap_or("52"),
    )?;
    for usage_index in 0..CLASSIC_USAGE_LINE_COUNT {
        let usage_name = classic_usage_frontend_name(usage_index);
        set_optional_message_value(
            message,
            &format!("{segment}.usage.{usage_name}"),
            job_param(job, &format!("DauerEdit5.usage.{usage_name}"), &usage_name),
        )?;
    }
    set_optional_message_value(
        message,
        &format!("{segment}.date"),
        job_param(job, "DauerEdit5.date", "date"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.orderid"),
        job,
        "DauerEdit5.orderid",
        "orderid",
        "DauerEdit requires orderid",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.firstdate"),
        job,
        "DauerEdit5.DauerDetails.firstdate",
        "firstdate",
        "DauerEdit requires firstdate",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.timeunit"),
        job,
        "DauerEdit5.DauerDetails.timeunit",
        "timeunit",
        "DauerEdit requires timeunit",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.turnus"),
        job,
        "DauerEdit5.DauerDetails.turnus",
        "turnus",
        "DauerEdit requires turnus",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.execday"),
        job,
        "DauerEdit5.DauerDetails.execday",
        "execday",
        "DauerEdit requires execday",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.DauerDetails.lastdate"),
        job_param(job, "DauerEdit5.DauerDetails.lastdate", "lastdate"),
    )?;

    Ok(())
}

fn render_dauer_del(
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
    let lowlevel_segment = "DauerDel4";
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerDel requires src.number or a passport account for the current DauerDel4 renderer",
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    if !has_account_identity(&dst_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "DauerDel requires dst.number for the current DauerDel4 renderer",
        ));
    }

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_classic_national_account_values(message, &format!("{segment}.Other"), &dst_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.name"),
        job,
        "DauerDel4.name",
        "name",
        "DauerDel requires name",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.name2"),
        job_param(job, "DauerDel4.name2", "name2"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        "DauerDel4.BTG.value",
        "btg.value",
        "DauerDel requires btg.value",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        "DauerDel4.BTG.curr",
        "btg.curr",
        "DauerDel requires btg.curr",
    )?;
    message.set_value(
        &format!("{segment}.key"),
        job_param(job, "DauerDel4.key", "key").unwrap_or("52"),
    )?;
    for usage_index in 0..CLASSIC_USAGE_LINE_COUNT {
        let usage_name = classic_usage_frontend_name(usage_index);
        set_optional_message_value(
            message,
            &format!("{segment}.usage.{usage_name}"),
            job_param(job, &format!("DauerDel4.usage.{usage_name}"), &usage_name),
        )?;
    }
    set_optional_message_value(
        message,
        &format!("{segment}.date"),
        job_param(job, "DauerDel4.date", "date"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.orderid"),
        job,
        "DauerDel4.orderid",
        "orderid",
        "DauerDel requires orderid",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.firstdate"),
        job,
        "DauerDel4.DauerDetails.firstdate",
        "firstdate",
        "DauerDel requires firstdate",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.timeunit"),
        job,
        "DauerDel4.DauerDetails.timeunit",
        "timeunit",
        "DauerDel requires timeunit",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.turnus"),
        job,
        "DauerDel4.DauerDetails.turnus",
        "turnus",
        "DauerDel requires turnus",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.execday"),
        job,
        "DauerDel4.DauerDetails.execday",
        "execday",
        "DauerDel requires execday",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.DauerDetails.lastdate"),
        job_param(job, "DauerDel4.DauerDetails.lastdate", "lastdate"),
    )?;

    Ok(())
}

fn render_term_ueb(
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
    let lowlevel_segment = "TermUeb4";
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUeb requires src.number or a passport account for the current TermUeb4 renderer",
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    if !has_account_identity(&dst_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUeb requires dst.number for the current TermUeb4 renderer",
        ));
    }

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_classic_national_account_values(message, &format!("{segment}.Other"), &dst_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.name"),
        job,
        "TermUeb4.name",
        "name",
        "TermUeb requires name",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.name2"),
        job_param(job, "TermUeb4.name2", "name2"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        "TermUeb4.BTG.value",
        "btg.value",
        "TermUeb requires btg.value",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        "TermUeb4.BTG.curr",
        "btg.curr",
        "TermUeb requires btg.curr",
    )?;
    message.set_value(
        &format!("{segment}.key"),
        job_param(job, "TermUeb4.key", "key").unwrap_or("51"),
    )?;
    for usage_index in 0..CLASSIC_USAGE_LINE_COUNT {
        let usage_name = classic_usage_frontend_name(usage_index);
        set_optional_message_value(
            message,
            &format!("{segment}.usage.{usage_name}"),
            job_param(job, &format!("TermUeb4.usage.{usage_name}"), &usage_name),
        )?;
    }
    set_required_message_value_from_job(
        message,
        &format!("{segment}.date"),
        job,
        "TermUeb4.date",
        "date",
        "TermUeb requires date",
    )?;

    Ok(())
}

fn render_term_ueb_del(
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
    let lowlevel_segment = "TermUebDel3";
    let segment = format!("{root}.{lowlevel_segment}");
    let order_id = job_param_required(
        job,
        "TermUebDel3.id",
        "orderid",
        "TermUebDel requires orderid",
    )?;
    let snapshot_key = format!("termueb_{order_id}");
    let snapshot = passport.get_persistent_data(&snapshot_key).ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("TermUebDel requires persistent data for {snapshot_key}"),
        )
    })?;

    for suffix in CLASSIC_INLAND_USER4_SNAPSHOT_SUFFIXES {
        if let Some(value) = snapshot.get(*suffix).filter(|value| !value.is_empty()) {
            message.set_value(&format!("{segment}.{suffix}"), value)?;
        }
    }
    message.set_value(&format!("{segment}.id"), order_id)?;

    Ok(())
}

fn render_term_ueb_edit(
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
    let lowlevel_segment = "TermUebEdit4";
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebEdit requires src.number or a passport account for the current TermUebEdit4 renderer",
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    if !has_account_identity(&dst_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebEdit requires dst.number for the current TermUebEdit4 renderer",
        ));
    }

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_classic_national_account_values(message, &format!("{segment}.Other"), &dst_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.name"),
        job,
        "TermUebEdit4.name",
        "name",
        "TermUebEdit requires name",
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.name2"),
        job_param(job, "TermUebEdit4.name2", "name2"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        "TermUebEdit4.BTG.value",
        "btg.value",
        "TermUebEdit requires btg.value",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        "TermUebEdit4.BTG.curr",
        "btg.curr",
        "TermUebEdit requires btg.curr",
    )?;
    message.set_value(
        &format!("{segment}.key"),
        job_param(job, "TermUebEdit4.key", "key").unwrap_or("51"),
    )?;
    for usage_index in 0..CLASSIC_USAGE_LINE_COUNT {
        let usage_name = classic_usage_frontend_name(usage_index);
        set_optional_message_value(
            message,
            &format!("{segment}.usage.{usage_name}"),
            job_param(
                job,
                &format!("TermUebEdit4.usage.{usage_name}"),
                &usage_name,
            ),
        )?;
    }
    set_required_message_value_from_job(
        message,
        &format!("{segment}.date"),
        job,
        "TermUebEdit4.date",
        "date",
        "TermUebEdit requires date",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.id"),
        job,
        "TermUebEdit4.id",
        "orderid",
        "TermUebEdit requires orderid",
    )?;

    Ok(())
}

fn render_ueb(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_classic_ueb(
        message,
        job,
        index,
        passport,
        ClassicUebRenderSpec {
            lowlevel_segment: "Ueb5",
            job_name: "Ueb",
            key_default: "51",
            first_usage_frontend: "usage",
        },
    )
}

fn render_ueb_bzu(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_classic_ueb(
        message,
        job,
        index,
        passport,
        ClassicUebRenderSpec {
            lowlevel_segment: "Ueb5",
            job_name: "UebBZU",
            key_default: "67",
            first_usage_frontend: "bzudata",
        },
    )
}

fn render_ueb_eil(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_classic_ueb(
        message,
        job,
        index,
        passport,
        ClassicUebRenderSpec {
            lowlevel_segment: "UebEil1",
            job_name: "UebEil",
            key_default: "51",
            first_usage_frontend: "usage",
        },
    )
}

fn render_ueb_foreign(
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
    let lowlevel_segment = "UebForeign2";
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "UebForeign requires src.number or a passport account for the current UebForeign2 renderer",
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.myname"),
        job,
        "UebForeign2.myname",
        "src.name",
        "UebForeign requires src.name",
    )?;
    set_optional_classic_national_account_values(
        message,
        &format!("{segment}.Other"),
        &dst_account,
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.otheriban"),
        job_param(job, "UebForeign2.otheriban", "dst.iban"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.otherkiname"),
        job,
        "UebForeign2.otherkiname",
        "dst.kiname",
        "UebForeign requires dst.kiname",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.othername"),
        job,
        "UebForeign2.othername",
        "dst.name",
        "UebForeign requires dst.name",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        "UebForeign2.BTG.value",
        "btg.value",
        "UebForeign requires btg.value",
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        "UebForeign2.BTG.curr",
        "btg.curr",
        "UebForeign requires btg.curr",
    )?;
    message.set_value(
        &format!("{segment}.kostentraeger"),
        job_param(job, "UebForeign2.kostentraeger", "kostentraeger").unwrap_or("1"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.usage"),
        job_param(job, "UebForeign2.usage", "usage"),
    )?;

    Ok(())
}

fn render_umb(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_classic_ueb(
        message,
        job,
        index,
        passport,
        ClassicUebRenderSpec {
            lowlevel_segment: "Umb2",
            job_name: "Umb",
            key_default: "51",
            first_usage_frontend: "usage",
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct ClassicUebRenderSpec {
    lowlevel_segment: &'static str,
    job_name: &'static str,
    key_default: &'static str,
    first_usage_frontend: &'static str,
}

fn render_classic_ueb(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
    spec: ClassicUebRenderSpec,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let lowlevel_segment = spec.lowlevel_segment;
    let job_name = spec.job_name;
    let segment = format!("{root}.{lowlevel_segment}");
    let src_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    if !has_account_identity(&src_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!(
                "{job_name} requires src.number or a passport account for the current {lowlevel_segment} renderer"
            ),
        ));
    }
    let dst_account = classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    if !has_account_identity(&dst_account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("{job_name} requires dst.number for the current {lowlevel_segment} renderer"),
        ));
    }

    set_classic_national_account_values(message, &format!("{segment}.My"), &src_account)?;
    set_classic_national_account_values(message, &format!("{segment}.Other"), &dst_account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.name"),
        job,
        &format!("{lowlevel_segment}.name"),
        "name",
        &format!("{job_name} requires name"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.name2"),
        job_param(job, &format!("{lowlevel_segment}.name2"), "name2"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.value"),
        job,
        &format!("{lowlevel_segment}.BTG.value"),
        "btg.value",
        &format!("{job_name} requires btg.value"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.BTG.curr"),
        job,
        &format!("{lowlevel_segment}.BTG.curr"),
        "btg.curr",
        &format!("{job_name} requires btg.curr"),
    )?;
    message.set_value(
        &format!("{segment}.key"),
        job_param(job, &format!("{lowlevel_segment}.key"), "key").unwrap_or(spec.key_default),
    )?;
    for usage_index in 0..CLASSIC_USAGE_LINE_COUNT {
        let usage_name = classic_usage_frontend_name(usage_index);
        let frontend_name = if usage_index == 0 {
            spec.first_usage_frontend
        } else {
            &usage_name
        };
        set_optional_message_value(
            message,
            &format!("{segment}.usage.{usage_name}"),
            job_param(
                job,
                &format!("{lowlevel_segment}.usage.{usage_name}"),
                frontend_name,
            ),
        )?;
    }

    Ok(())
}

fn render_dauer_sepa_new(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_dauer_sepa_order_job(
        message,
        job,
        index,
        passport,
        DauerSepaOrderRenderSpec {
            job_name: "DauerSEPANew",
            lowlevel_segment: "DauerSEPANew1",
            sepa_descriptor: PAIN_001_001_02_URN,
            include_order_id: false,
            include_date: false,
        },
    )
}

fn render_dauer_last_sepa_new(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_dauer_sepa_order_job(
        message,
        job,
        index,
        passport,
        DauerSepaOrderRenderSpec {
            job_name: "DauerLastSEPANew",
            lowlevel_segment: "DauerLastSEPANew1",
            sepa_descriptor: PAIN_008_001_01_URN,
            include_order_id: false,
            include_date: false,
        },
    )
}

fn render_ueb_sepa(
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
    let lowlevel_segment = "UebSEPA1";
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "UebSEPA requires src.iban, src.number, or a passport account for the current UebSEPA1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "UebSEPA1.sepadescr", "_sepadescriptor").unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        "UebSEPA1.sepapain",
        "_sepapain",
        "UebSEPA requires _sepapain or SEPA parameters for PAIN generation",
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_multi_ueb_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_multi_ueb_sepa_job(
        message,
        job,
        index,
        passport,
        "MultiUebSEPA",
        "SammelUebSEPA1",
    )
}

fn render_term_multi_ueb_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_multi_ueb_sepa_job(
        message,
        job,
        index,
        passport,
        "TermMultiUebSEPA",
        "TermSammelUebSEPA1",
    )
}

fn render_multi_ueb_sepa_job(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
    job_name: &str,
    lowlevel_segment: &str,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!(
                "{job_name} requires src.iban, src.number, or a passport account for the current {lowlevel_segment} renderer"
            ),
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.Total.value"),
        job,
        &format!("{lowlevel_segment}.Total.value"),
        "Total.value",
        &format!("{job_name} requires Total.value or generated SEPA parameters"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.Total.curr"),
        job,
        &format!("{lowlevel_segment}.Total.curr"),
        "Total.curr",
        &format!("{job_name} requires Total.curr or generated SEPA parameters"),
    )?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(
            job,
            &format!("{lowlevel_segment}.sepadescr"),
            "_sepadescriptor",
        )
        .unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        &format!("{lowlevel_segment}.sepapain"),
        "_sepapain",
        &format!("{job_name} requires _sepapain or SEPA parameters for PAIN generation"),
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_last_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_last_direct_debit_sepa(
        message,
        job,
        index,
        passport,
        LastDirectDebitSepaRenderSpec {
            job_name: "LastSEPA",
            lowlevel_segment: "LastSEPA1",
        },
    )
}

fn render_last_cor1_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_last_direct_debit_sepa(
        message,
        job,
        index,
        passport,
        LastDirectDebitSepaRenderSpec {
            job_name: "LastCOR1SEPA",
            lowlevel_segment: "LastCOR1SEPA1",
        },
    )
}

fn render_last_b2b_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_last_direct_debit_sepa(
        message,
        job,
        index,
        passport,
        LastDirectDebitSepaRenderSpec {
            job_name: "LastB2BSEPA",
            lowlevel_segment: "LastB2BSEPA1",
        },
    )
}

fn render_multi_last_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_multi_last_direct_debit_sepa(
        message,
        job,
        index,
        passport,
        LastDirectDebitSepaRenderSpec {
            job_name: "MultiLastSEPA",
            lowlevel_segment: "SammelLastSEPA1",
        },
    )
}

fn render_multi_last_cor1_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_multi_last_direct_debit_sepa(
        message,
        job,
        index,
        passport,
        LastDirectDebitSepaRenderSpec {
            job_name: "MultiLastCOR1SEPA",
            lowlevel_segment: "SammelLastCOR1SEPA1",
        },
    )
}

fn render_multi_last_b2b_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_multi_last_direct_debit_sepa(
        message,
        job,
        index,
        passport,
        LastDirectDebitSepaRenderSpec {
            job_name: "MultiLastB2BSEPA",
            lowlevel_segment: "SammelLastB2BSEPA1",
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct LastDirectDebitSepaRenderSpec {
    job_name: &'static str,
    lowlevel_segment: &'static str,
}

fn render_multi_last_direct_debit_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
    spec: LastDirectDebitSepaRenderSpec,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let job_name = spec.job_name;
    let lowlevel_segment = spec.lowlevel_segment;
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!(
                "{job_name} requires src.iban, src.number, or a passport account for the current {lowlevel_segment} renderer"
            ),
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.Total.value"),
        job,
        &format!("{lowlevel_segment}.Total.value"),
        "Total.value",
        &format!("{job_name} requires Total.value or generated SEPA parameters"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.Total.curr"),
        job,
        &format!("{lowlevel_segment}.Total.curr"),
        "Total.curr",
        &format!("{job_name} requires Total.curr or generated SEPA parameters"),
    )?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(
            job,
            &format!("{lowlevel_segment}.sepadescr"),
            "_sepadescriptor",
        )
        .unwrap_or(PAIN_008_001_01_URN),
    )?;
    let sepapain = job_param_required(
        job,
        &format!("{lowlevel_segment}.sepapain"),
        "_sepapain",
        &format!("{job_name} requires _sepapain or SEPA parameters for PAIN generation"),
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_last_direct_debit_sepa(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
    spec: LastDirectDebitSepaRenderSpec,
) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let job_name = spec.job_name;
    let lowlevel_segment = spec.lowlevel_segment;
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!(
                "{job_name} requires src.iban, src.number, or a passport account for the current {lowlevel_segment} renderer"
            ),
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(
            job,
            &format!("{lowlevel_segment}.sepadescr"),
            "_sepadescriptor",
        )
        .unwrap_or(PAIN_008_001_01_URN),
    )?;
    let sepapain = job_param_required(
        job,
        &format!("{lowlevel_segment}.sepapain"),
        "_sepapain",
        &format!("{job_name} requires _sepapain or SEPA parameters for PAIN generation"),
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_inst_ueb_sepa(
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
    let lowlevel_segment = "InstUebSEPA1";
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "InstUebSEPA requires src.iban, src.number, or a passport account for the current InstUebSEPA1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "InstUebSEPA1.sepadescr", "_sepadescriptor").unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        "InstUebSEPA1.sepapain",
        "_sepapain",
        "InstUebSEPA requires _sepapain or SEPA parameters for PAIN generation",
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_umb_sepa(
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
    let lowlevel_segment = "UmbSEPA1";
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "UmbSEPA requires src.iban, src.number, or a passport account for the current UmbSEPA1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "UmbSEPA1.sepadescr", "_sepadescriptor").unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        "UmbSEPA1.sepapain",
        "_sepapain",
        "UmbSEPA requires _sepapain or SEPA parameters for PAIN generation",
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_term_ueb_sepa(
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
    let lowlevel_segment = "TermUebSEPA1";
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebSEPA requires src.iban, src.number, or a passport account for the current TermUebSEPA1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "TermUebSEPA1.sepadescr", "_sepadescriptor").unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        "TermUebSEPA1.sepapain",
        "_sepapain",
        "TermUebSEPA requires _sepapain or SEPA parameters for PAIN generation",
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    Ok(())
}

fn render_term_ueb_sepa_del(
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
    let lowlevel_segment = "TermUebSEPADel1";
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebSEPADel requires src.iban, src.number, or a passport account for the current TermUebSEPADel1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "TermUebSEPADel1.sepadescr", "_sepadescriptor")
            .unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        "TermUebSEPADel1.sepapain",
        "_sepapain",
        "TermUebSEPADel requires _sepapain or SEPA parameters for PAIN generation",
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.orderid"),
        job,
        "TermUebSEPADel1.orderid",
        "orderid",
        "TermUebSEPADel requires orderid",
    )?;

    Ok(())
}

fn render_term_ueb_sepa_edit(
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
    let lowlevel_segment = "TermUebSEPAEdit1";
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "TermUebSEPAEdit requires src.iban, src.number, or a passport account for the current TermUebSEPAEdit1 renderer",
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(job, "TermUebSEPAEdit1.sepadescr", "_sepadescriptor")
            .unwrap_or(PAIN_001_001_02_URN),
    )?;
    let sepapain = job_param_required(
        job,
        "TermUebSEPAEdit1.sepapain",
        "_sepapain",
        "TermUebSEPAEdit requires _sepapain or SEPA parameters for PAIN generation",
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.orderid"),
        job,
        "TermUebSEPAEdit1.orderid",
        "orderid",
        "TermUebSEPAEdit requires orderid",
    )?;

    Ok(())
}

fn render_dauer_sepa_edit(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_dauer_sepa_order_job(
        message,
        job,
        index,
        passport,
        DauerSepaOrderRenderSpec {
            job_name: "DauerSEPAEdit",
            lowlevel_segment: "DauerSEPAEdit1",
            sepa_descriptor: PAIN_001_001_02_URN,
            include_order_id: true,
            include_date: true,
        },
    )
}

fn render_dauer_sepa_del(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
) -> HbciResult<()> {
    render_dauer_sepa_order_job(
        message,
        job,
        index,
        passport,
        DauerSepaOrderRenderSpec {
            job_name: "DauerSEPADel",
            lowlevel_segment: "DauerSEPADel1",
            sepa_descriptor: PAIN_001_001_02_URN,
            include_order_id: true,
            include_date: true,
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct DauerSepaOrderRenderSpec {
    job_name: &'static str,
    lowlevel_segment: &'static str,
    sepa_descriptor: &'static str,
    include_order_id: bool,
    include_date: bool,
}

fn render_dauer_sepa_order_job(
    message: &mut HbciMessage,
    job: &HbciJob,
    index: usize,
    passport: &PinTanPassport,
    spec: DauerSepaOrderRenderSpec,
) -> HbciResult<()> {
    let job_name = spec.job_name;
    let lowlevel_segment = spec.lowlevel_segment;
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.{lowlevel_segment}");
    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!(
                "{job_name} requires src.iban, src.number, or a passport account for the current {lowlevel_segment} renderer"
            ),
        ));
    }

    set_ktv_int_account_values(message, &format!("{segment}.My"), &account)?;
    message.set_value(
        &format!("{segment}.sepadescr"),
        job_param(
            job,
            &format!("{lowlevel_segment}.sepadescr"),
            "_sepadescriptor",
        )
        .unwrap_or(spec.sepa_descriptor),
    )?;
    let sepapain = job_param_required(
        job,
        &format!("{lowlevel_segment}.sepapain"),
        "_sepapain",
        &format!("{job_name} requires _sepapain or SEPA parameters for PAIN generation"),
    )?;
    message.set_value(&format!("{segment}.sepapain"), sepa_binary_value(sepapain))?;

    if spec.include_date {
        set_optional_message_value(
            message,
            &format!("{segment}.date"),
            job_param(job, &format!("{lowlevel_segment}.date"), "date"),
        )?;
    }
    if spec.include_order_id {
        set_required_message_value_from_job(
            message,
            &format!("{segment}.orderid"),
            job,
            &format!("{lowlevel_segment}.orderid"),
            "orderid",
            &format!("{job_name} requires orderid"),
        )?;
    }

    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.firstdate"),
        job,
        &format!("{lowlevel_segment}.DauerDetails.firstdate"),
        "firstdate",
        &format!("{job_name} requires firstdate"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.timeunit"),
        job,
        &format!("{lowlevel_segment}.DauerDetails.timeunit"),
        "timeunit",
        &format!("{job_name} requires timeunit"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.turnus"),
        job,
        &format!("{lowlevel_segment}.DauerDetails.turnus"),
        "turnus",
        &format!("{job_name} requires turnus"),
    )?;
    set_required_message_value_from_job(
        message,
        &format!("{segment}.DauerDetails.execday"),
        job,
        &format!("{lowlevel_segment}.DauerDetails.execday"),
        "execday",
        &format!("{job_name} requires execday"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.DauerDetails.lastdate"),
        job_param(
            job,
            &format!("{lowlevel_segment}.DauerDetails.lastdate"),
            "lastdate",
        ),
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

fn render_kums_all_camt(
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
    let segment = format!("{root}.KUmsZeitCamt1");
    let account = effective_job_account(job, passport, "KUmsZeitCamt1", "my");
    if !has_account_identity(&account) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "KUmsAllCamt requires my.iban, my.number, or a passport account for the current KUmsZeitCamt1 tracer renderer",
        ));
    }

    set_account_values(message, &segment, &account)?;
    message.set_value(
        &format!("{segment}.formats.suppformat"),
        job_param(job, "KUmsZeitCamt1.formats.suppformat", "suppformat")
            .unwrap_or(CAMT_052_001_01_URN),
    )?;
    message.set_value(
        &format!("{segment}.allaccounts"),
        job_param(job, "KUmsZeitCamt1.allaccounts", "dummy").unwrap_or("N"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.startdate"),
        job_param(job, "KUmsZeitCamt1.startdate", "startdate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.enddate"),
        job_param(job, "KUmsZeitCamt1.enddate", "enddate"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.maxentries"),
        job_param(job, "KUmsZeitCamt1.maxentries", "maxentries"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.offset"),
        job_param(job, "KUmsZeitCamt1.offset", "offset"),
    )?;

    Ok(())
}

fn render_sepa_info(message: &mut HbciMessage, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    message.set_value(&format!("{root}.SEPAInfo1"), "requested")
}

fn render_receipt(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.Receipt1");
    let receipt = job_param_required(
        job,
        "Receipt1.receipt",
        "receipt",
        "Receipt requires receipt",
    )?;

    message.set_value(&segment, "requested")?;
    message.set_value(&format!("{segment}.receipt"), sepa_binary_value(receipt))
}

fn render_tan_media_list(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.TANMediaList4");

    message.set_value(
        &format!("{segment}.mediatype"),
        job_param(job, "TANMediaList4.mediatype", "mediatype").unwrap_or("0"),
    )?;
    message.set_value(
        &format!("{segment}.mediacategory"),
        job_param(job, "TANMediaList4.mediacategory", "mediacategory").unwrap_or("A"),
    )?;

    Ok(())
}

fn render_tan2step(message: &mut HbciMessage, job: &HbciJob, index: usize) -> HbciResult<()> {
    let root = if index == 0 {
        "CustomMsg.GV".to_owned()
    } else {
        format!("CustomMsg.GV_{}", index + 1)
    };
    let segment = format!("{root}.TAN2Step5");

    message.set_value(&segment, "requested")?;
    message.set_value(
        &format!("{segment}.process"),
        job_param_required(
            job,
            "TAN2Step5.process",
            "process",
            "TAN2Step requires process",
        )?,
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.ordersegcode"),
        job_param(job, "TAN2Step5.ordersegcode", "ordersegcode"),
    )?;
    set_tan_order_account_values(message, &segment, job)?;
    set_optional_message_value(
        message,
        &format!("{segment}.orderhash"),
        tan_orderhash_param(job).as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.orderref"),
        job_param(job, "TAN2Step5.orderref", "orderref"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.listidx"),
        job_param(job, "TAN2Step5.listidx", "listidx"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.notlasttan"),
        job_param(job, "TAN2Step5.notlasttan", "notlasttan"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.storno"),
        job_param(job, "TAN2Step5.storno", "storno"),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.challengeklass"),
        job_param(job, "TAN2Step5.challengeklass", "challengeklass"),
    )?;
    for index in 1..=9 {
        set_optional_message_value(
            message,
            &format!("{segment}.ChallengeKlassParams.param{index}"),
            job_param(
                job,
                &format!("TAN2Step5.ChallengeKlassParams.param{index}"),
                &format!("ChallengeKlassParam{index}"),
            ),
        )?;
    }
    set_optional_message_value(
        message,
        &format!("{segment}.tanmedia"),
        job_param(job, "TAN2Step5.tanmedia", "tanmedia"),
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

fn effective_job_my_account(
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
            &format!("{lowlevel_segment}.My.iban"),
            &format!("{frontend_base}.iban"),
        ),
    );
    overlay_account_param(
        &mut account.bic,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.bic"),
            &format!("{frontend_base}.bic"),
        ),
    );
    overlay_account_param(
        &mut account.country,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.KIK.country"),
            &format!("{frontend_base}.country"),
        ),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.KIK.blz"),
            &format!("{frontend_base}.blz"),
        ),
    );
    overlay_account_param(
        &mut account.number,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.number"),
            &format!("{frontend_base}.number"),
        ),
    );
    overlay_account_param(
        &mut account.subnumber,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.subnumber"),
            &format!("{frontend_base}.subnumber"),
        ),
    );

    account
}

fn classic_national_job_account(
    job: &HbciJob,
    fallback: Option<Konto>,
    lowlevel_segment: &str,
    group_name: &str,
    frontend_base: &str,
) -> Konto {
    let mut account = fallback.unwrap_or_default();

    overlay_account_param(
        &mut account.country,
        job_param(
            job,
            &format!("{lowlevel_segment}.{group_name}.KIK.country"),
            &format!("{frontend_base}.country"),
        ),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(
            job,
            &format!("{lowlevel_segment}.{group_name}.KIK.blz"),
            &format!("{frontend_base}.blz"),
        ),
    );
    overlay_account_param(
        &mut account.number,
        job_param(
            job,
            &format!("{lowlevel_segment}.{group_name}.number"),
            &format!("{frontend_base}.number"),
        ),
    );
    overlay_account_param(
        &mut account.subnumber,
        job_param(
            job,
            &format!("{lowlevel_segment}.{group_name}.subnumber"),
            &format!("{frontend_base}.subnumber"),
        ),
    );

    account
}

fn dauer_sepa_list_account(job: &HbciJob, passport: &PinTanPassport) -> Konto {
    let mut account = passport.first_account().cloned().unwrap_or_default();

    overlay_account_param(
        &mut account.iban,
        job.lowlevel_param("DauerSEPAList2.My.iban")
            .or_else(|| job.param("my.iban"))
            .or_else(|| job.param("src.iban"))
            .filter(|value| !value.is_empty()),
    );
    overlay_account_param(
        &mut account.bic,
        job.lowlevel_param("DauerSEPAList2.My.bic")
            .or_else(|| job.param("my.bic"))
            .or_else(|| job.param("src.bic"))
            .filter(|value| !value.is_empty()),
    );
    overlay_account_param(
        &mut account.country,
        job_param(job, "DauerSEPAList2.My.KIK.country", "my.country"),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(job, "DauerSEPAList2.My.KIK.blz", "my.blz"),
    );
    overlay_account_param(
        &mut account.number,
        job_param(job, "DauerSEPAList2.My.number", "my.number"),
    );
    overlay_account_param(
        &mut account.subnumber,
        job_param(job, "DauerSEPAList2.My.subnumber", "my.subnumber"),
    );

    account
}

fn term_ueb_sepa_list_account(job: &HbciJob, passport: &PinTanPassport) -> Konto {
    let mut account = passport.first_account().cloned().unwrap_or_default();

    overlay_account_param(
        &mut account.iban,
        job.lowlevel_param("TermUebSEPAList1.My.iban")
            .or_else(|| job.param("my.iban"))
            .or_else(|| job.param("src.iban"))
            .filter(|value| !value.is_empty()),
    );
    overlay_account_param(
        &mut account.bic,
        job.lowlevel_param("TermUebSEPAList1.My.bic")
            .or_else(|| job.param("my.bic"))
            .or_else(|| job.param("src.bic"))
            .filter(|value| !value.is_empty()),
    );
    overlay_account_param(
        &mut account.country,
        job_param(job, "TermUebSEPAList1.My.KIK.country", "my.country"),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(job, "TermUebSEPAList1.My.KIK.blz", "my.blz"),
    );
    overlay_account_param(
        &mut account.number,
        job_param(job, "TermUebSEPAList1.My.number", "my.number"),
    );
    overlay_account_param(
        &mut account.subnumber,
        job_param(job, "TermUebSEPAList1.My.subnumber", "my.subnumber"),
    );

    account
}

fn wp_depot_list_account(job: &HbciJob, passport: &PinTanPassport) -> HbciResult<Konto> {
    let number = job_param_required(
        job,
        "WPDepotList6.Depot.number",
        "my.number",
        "WPDepotList requires my.number",
    )?;
    let mut account = passport.account_by_number(number);

    overlay_account_param(
        &mut account.subnumber,
        job_param(job, "WPDepotList6.Depot.subnumber", "my.subnumber"),
    );
    overlay_account_param(
        &mut account.country,
        job_param(job, "WPDepotList6.Depot.KIK.country", "my.country"),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(job, "WPDepotList6.Depot.KIK.blz", "my.blz"),
    );

    Ok(account)
}

fn wp_depot_ums_account(job: &HbciJob, passport: &PinTanPassport) -> HbciResult<Konto> {
    let number = job_param_required(
        job,
        "WPDepotUms5.Depot.number",
        "my.number",
        "WPDepotUms requires my.number",
    )?;
    let mut account = passport.account_by_number(number);

    overlay_account_param(
        &mut account.subnumber,
        job_param(job, "WPDepotUms5.Depot.subnumber", "my.subnumber"),
    );
    overlay_account_param(
        &mut account.country,
        job_param(job, "WPDepotUms5.Depot.KIK.country", "my.country"),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(job, "WPDepotUms5.Depot.KIK.blz", "my.blz"),
    );

    Ok(account)
}

fn standing_order_sepa_account(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Konto {
    let mut account = passport.first_account().cloned().unwrap_or_default();

    overlay_account_param(
        &mut account.iban,
        job_param(job, &format!("{lowlevel_segment}.My.iban"), "src.iban"),
    );
    overlay_account_param(
        &mut account.bic,
        job_param(job, &format!("{lowlevel_segment}.My.bic"), "src.bic"),
    );
    overlay_account_param(
        &mut account.country,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.KIK.country"),
            "src.country",
        ),
    );
    overlay_account_param(
        &mut account.blz,
        job_param(job, &format!("{lowlevel_segment}.My.KIK.blz"), "src.blz"),
    );
    overlay_account_param(
        &mut account.number,
        job_param(job, &format!("{lowlevel_segment}.My.number"), "src.number"),
    );
    overlay_account_param(
        &mut account.subnumber,
        job_param(
            job,
            &format!("{lowlevel_segment}.My.subnumber"),
            "src.subnumber",
        ),
    );

    account
}

fn job_param<'a>(job: &'a HbciJob, lowlevel_name: &str, frontend_name: &str) -> Option<&'a str> {
    job.lowlevel_param(lowlevel_name)
        .or_else(|| job.param(frontend_name))
        .filter(|value| !value.is_empty())
}

fn job_param_required<'a>(
    job: &'a HbciJob,
    lowlevel_name: &str,
    frontend_name: &str,
    message: &str,
) -> HbciResult<&'a str> {
    job_param(job, lowlevel_name, frontend_name)
        .ok_or_else(|| HbciError::new(HbciErrorKind::InvalidArgument, message))
}

fn apply_term_ueb_snapshot_to_job(
    job: &mut HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> HbciResult<()> {
    let order_id = job
        .lowlevel_param(&format!("{lowlevel_segment}.id"))
        .or_else(|| job.param("orderid"))
        .filter(|value| !value.is_empty())
        .map(|value| value.to_owned());
    let Some(order_id) = order_id else {
        return Ok(());
    };
    let snapshot_key = format!("termueb_{order_id}");
    let snapshot = passport.get_persistent_data(&snapshot_key).ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("{} requires persistent data for {snapshot_key}", job.name()),
        )
    })?;

    for suffix in CLASSIC_INLAND_USER4_SNAPSHOT_SUFFIXES {
        if let Some(value) = snapshot.get(*suffix).filter(|value| !value.is_empty()) {
            job.set_lowlevel_param_if_absent(format!("{lowlevel_segment}.{suffix}"), value.clone());
        }
    }

    Ok(())
}

fn apply_dauer_snapshot_to_job(
    job: &mut HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> HbciResult<()> {
    let order_id = job
        .lowlevel_param(&format!("{lowlevel_segment}.orderid"))
        .or_else(|| job.param("orderid"))
        .filter(|value| !value.is_empty())
        .map(|value| value.to_owned());
    let Some(order_id) = order_id else {
        return Ok(());
    };
    let snapshot_key = format!("dauer_{order_id}");
    let Some(snapshot) = passport.get_persistent_data(&snapshot_key) else {
        return Ok(());
    };

    for (key, value) in snapshot {
        if value.is_empty() || key == "date" || key.starts_with("Aussetzung.") {
            continue;
        }
        job.set_lowlevel_param_if_absent(format!("{lowlevel_segment}.{key}"), value.clone());
    }

    Ok(())
}

fn set_required_message_value_from_job(
    message: &mut HbciMessage,
    path: &str,
    job: &HbciJob,
    lowlevel_name: &str,
    frontend_name: &str,
    error_message: &str,
) -> HbciResult<()> {
    let value = job_param_required(job, lowlevel_name, frontend_name, error_message)?;
    message.set_value(path, value)
}

fn sepa_binary_value(value: &str) -> String {
    if value.starts_with('B') || value.starts_with('N') {
        value.to_owned()
    } else {
        format!("B{value}")
    }
}

fn tan_orderhash_param(job: &HbciJob) -> Option<String> {
    if let Some(value) = job
        .lowlevel_param("TAN2Step5.orderhash")
        .filter(|value| !value.is_empty())
    {
        return Some(value.to_owned());
    }

    job.param("orderhash")
        .filter(|value| !value.is_empty())
        .map(|value| format!("B{value}"))
}

fn set_tan_order_account_values(
    message: &mut HbciMessage,
    segment: &str,
    job: &HbciJob,
) -> HbciResult<()> {
    let account = tan_order_account(job);
    if !has_account_identity(&account) {
        return Ok(());
    }

    set_optional_message_value(
        message,
        &format!("{segment}.OrderAccount.iban"),
        account.iban.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.OrderAccount.bic"),
        account.bic.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.OrderAccount.number"),
        account.number.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.OrderAccount.subnumber"),
        account.subnumber.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.OrderAccount.KIK.country"),
        account.country.as_deref().or(Some("DE")),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.OrderAccount.KIK.blz"),
        account.blz.as_deref(),
    )?;
    Ok(())
}

fn tan_order_account(job: &HbciJob) -> Konto {
    Konto {
        iban: job_param(job, "TAN2Step5.OrderAccount.iban", "orderaccount.iban")
            .map(ToOwned::to_owned),
        bic: job_param(job, "TAN2Step5.OrderAccount.bic", "orderaccount.bic")
            .map(ToOwned::to_owned),
        number: job_param(job, "TAN2Step5.OrderAccount.number", "orderaccount.number")
            .map(ToOwned::to_owned),
        subnumber: job_param(
            job,
            "TAN2Step5.OrderAccount.subnumber",
            "orderaccount.subnumber",
        )
        .map(ToOwned::to_owned),
        country: job_param(
            job,
            "TAN2Step5.OrderAccount.KIK.country",
            "orderaccount.country",
        )
        .map(ToOwned::to_owned),
        blz: job_param(job, "TAN2Step5.OrderAccount.KIK.blz", "orderaccount.blz")
            .map(ToOwned::to_owned),
        ..Konto::default()
    }
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

fn classic_usage_frontend_name(index: usize) -> String {
    if index == 0 {
        "usage".to_owned()
    } else {
        format!("usage_{}", index + 1)
    }
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

fn set_national_account_values(
    message: &mut HbciMessage,
    segment: &str,
    account: &Konto,
) -> HbciResult<()> {
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
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.KIK.country"),
        account.country.as_deref().or(Some("DE")),
    )?;
    set_optional_message_value(
        message,
        &format!("{segment}.KTV.KIK.blz"),
        account.blz.as_deref(),
    )?;
    Ok(())
}

fn set_classic_national_account_values(
    message: &mut HbciMessage,
    prefix: &str,
    account: &Konto,
) -> HbciResult<()> {
    set_optional_message_value(
        message,
        &format!("{prefix}.number"),
        account.number.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.subnumber"),
        account.subnumber.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.KIK.country"),
        account.country.as_deref().or(Some("DE")),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.KIK.blz"),
        account.blz.as_deref(),
    )?;
    Ok(())
}

fn set_optional_classic_national_account_values(
    message: &mut HbciMessage,
    prefix: &str,
    account: &Konto,
) -> HbciResult<()> {
    set_optional_message_value(
        message,
        &format!("{prefix}.number"),
        account.number.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.subnumber"),
        account.subnumber.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.KIK.country"),
        account.country.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.KIK.blz"),
        account.blz.as_deref(),
    )?;
    Ok(())
}

fn set_ktv_int_account_values(
    message: &mut HbciMessage,
    prefix: &str,
    account: &Konto,
) -> HbciResult<()> {
    set_optional_message_value(message, &format!("{prefix}.iban"), account.iban.as_deref())?;
    set_optional_message_value(message, &format!("{prefix}.bic"), account.bic.as_deref())?;
    set_optional_message_value(
        message,
        &format!("{prefix}.number"),
        account.number.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.subnumber"),
        account.subnumber.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.KIK.country"),
        account.country.as_deref(),
    )?;
    set_optional_message_value(
        message,
        &format!("{prefix}.KIK.blz"),
        account.blz.as_deref(),
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
            "AccInfo" => self
                .acc_info_result_for_root(acc_info_response_root(index))
                .map(HbciJobResultData::AccInfo),
            "CardList" => self
                .card_list_result_for_root(card_list_response_root(index))
                .map(HbciJobResultData::CardList),
            "DauerEdit" => self
                .dauer_edit_result_for_root(dauer_edit_response_root(index))
                .map(HbciJobResultData::DauerEdit),
            "DauerList" => self
                .dauer_list_result_for_root(dauer_list_response_root(index))
                .map(HbciJobResultData::DauerList),
            "DauerNew" => self
                .dauer_new_result_for_root(dauer_new_response_root(index))
                .map(HbciJobResultData::DauerNew),
            "DauerSEPADel" => self
                .dauer_edit_result_for_root(dauer_sepa_edit_response_root(index))
                .map(HbciJobResultData::DauerEdit),
            "DauerSEPAEdit" => self
                .dauer_edit_result_for_root(dauer_sepa_edit_response_root(index))
                .map(HbciJobResultData::DauerEdit),
            "DauerSEPAList" => self
                .dauer_list_result_for_root(dauer_sepa_list_response_root(index))
                .map(HbciJobResultData::DauerList),
            "DauerLastSEPAList" => self
                .dauer_last_sepa_list_result_for_root(dauer_last_sepa_list_response_root(index))
                .map(HbciJobResultData::DauerList),
            "DauerSEPANew" => self
                .dauer_new_result_for_root(dauer_sepa_new_response_root(index))
                .map(HbciJobResultData::DauerNew),
            "DauerLastSEPANew" => self
                .dauer_new_result_for_root(dauer_last_sepa_new_response_root(index))
                .map(HbciJobResultData::DauerNew),
            "FestCondList" => self
                .fest_cond_list_result_for_root(fest_cond_list_response_root(index))
                .map(HbciJobResultData::FestCondList),
            "FestList" => self
                .fest_list_result_for_root(fest_list_response_root(index))
                .map(HbciJobResultData::FestList),
            "InfoList" => self
                .info_list_result_for_root(info_list_response_root(index))
                .map(HbciJobResultData::InfoList),
            "InfoOrder" => self
                .info_order_result_for_root(info_order_response_root(index))
                .map(HbciJobResultData::InfoOrder),
            "InstUebSEPA" => self
                .inst_ueb_sepa_result_for_root(inst_ueb_sepa_response_root(index))
                .map(HbciJobResultData::InstUebSepa),
            "LastB2BSEPA" => self
                .last_sepa_result_for_root(last_b2b_sepa_response_root(index))
                .map(HbciJobResultData::LastSepa),
            "LastCOR1SEPA" => self
                .last_sepa_result_for_root(last_cor1_sepa_response_root(index))
                .map(HbciJobResultData::LastSepa),
            "LastSEPA" => self
                .last_sepa_result_for_root(last_sepa_response_root(index))
                .map(HbciJobResultData::LastSepa),
            "MultiLastB2BSEPA" => self
                .last_sepa_result_for_root(multi_last_b2b_sepa_response_root(index))
                .map(HbciJobResultData::LastSepa),
            "MultiLastCOR1SEPA" => self
                .last_sepa_result_for_root(multi_last_cor1_sepa_response_root(index))
                .map(HbciJobResultData::LastSepa),
            "MultiLastSEPA" => self
                .last_sepa_result_for_root(multi_last_sepa_response_root(index))
                .map(HbciJobResultData::LastSepa),
            "Kontoauszug" => self
                .kontoauszug_result_for_root(kontoauszug_response_root(index))
                .map(HbciJobResultData::Kontoauszug),
            "KontoauszugPdf" => self
                .kontoauszug_pdf_result_for_root(kontoauszug_pdf_response_root(index))
                .map(HbciJobResultData::Kontoauszug),
            "Status" => self.status_result().map(HbciJobResultData::Status),
            "TermUeb" => self
                .term_ueb_result_for_root(term_ueb_response_root(index))
                .map(HbciJobResultData::TermUeb),
            "TermUebEdit" => self
                .term_ueb_edit_result_for_root(term_ueb_edit_response_root(index))
                .map(HbciJobResultData::TermUebEdit),
            "TermUebSEPA" => self
                .term_ueb_result_for_root(term_ueb_sepa_response_root(index))
                .map(HbciJobResultData::TermUeb),
            "TermMultiUebSEPA" => self
                .term_ueb_result_for_root(term_multi_ueb_sepa_response_root(index))
                .map(HbciJobResultData::TermUeb),
            "TermUebSEPAEdit" => self
                .term_ueb_edit_result_for_root(term_ueb_sepa_edit_response_root(index))
                .map(HbciJobResultData::TermUebEdit),
            "TermUebList" => self
                .term_ueb_list_result_for_root(term_ueb_list_response_root(index))
                .map(HbciJobResultData::TermUebList),
            "TermUebSEPAList" => self
                .term_ueb_list_result_for_root(term_ueb_sepa_list_response_root(index))
                .map(HbciJobResultData::TermUebList),
            "SaldoReq" => self
                .saldo_result_for_index(index, passport)
                .map(HbciJobResultData::SaldoReq),
            "SaldoReqAll" => {
                let result = self.saldo_result_all(passport);
                (!result.entries.is_empty()).then_some(HbciJobResultData::SaldoReq(result))
            }
            "KUmsAll" => self.kums_result_for_root(kums_response_root("KUmsZeitRes7", index)),
            "KUmsZeitSEPA" => {
                self.kums_result_for_root(kums_response_root("KUmsZeitSEPARes7", index))
            }
            "KUmsAllCamt" => {
                self.kums_all_camt_result_for_root(kums_response_root("KUmsZeitCamtRes1", index))
            }
            "KUmsNew" => self.kums_result_for_root(kums_response_root("KUmsNewRes7", index)),
            "TANList" => self.tan_list_result().map(HbciJobResultData::TanList),
            "TANMediaList" => self
                .tan_media_list_result_for_root(tan_media_list_response_root(index))
                .map(HbciJobResultData::TanMediaList),
            "VoP" => self
                .vop_result_for_root(vop_response_root(index))
                .map(HbciJobResultData::VoP),
            "WPDepotList" => self
                .wp_depot_list_result_for_root(wp_depot_list_response_root(index))
                .map(HbciJobResultData::WPDepotList),
            "WPDepotUms" => self
                .wp_depot_ums_result_for_root(wp_depot_ums_response_root(index))
                .map(HbciJobResultData::WPDepotUms),
            _ => None,
        }
    }

    fn result_data_for_job(&self, job: &HbciJob, index: usize) -> BTreeMap<String, String> {
        match job.name() {
            "AccInfo" => self.content_result_data([acc_info_response_root(index)]),
            "CardList" => self.content_result_data([card_list_response_root(index)]),
            "DauerEdit" => self.content_result_data([dauer_edit_response_root(index)]),
            "DauerList" => self.content_result_data([dauer_list_response_root(index)]),
            "DauerNew" => self.content_result_data([dauer_new_response_root(index)]),
            "DauerSEPADel" => self.content_result_data([dauer_sepa_edit_response_root(index)]),
            "DauerSEPAEdit" => self.content_result_data([dauer_sepa_edit_response_root(index)]),
            "DauerSEPAList" => self.content_result_data([dauer_sepa_list_response_root(index)]),
            "DauerSEPANew" => self.content_result_data([dauer_sepa_new_response_root(index)]),
            "DauerLastSEPAList" => {
                self.content_result_data([dauer_last_sepa_list_response_root(index)])
            }
            "DauerLastSEPANew" => {
                self.content_result_data([dauer_last_sepa_new_response_root(index)])
            }
            "FestCondList" => self.content_result_data([fest_cond_list_response_root(index)]),
            "FestList" => self.content_result_data([fest_list_response_root(index)]),
            "InfoList" => self.content_result_data([info_list_response_root(index)]),
            "InfoOrder" => self.content_result_data([info_order_response_root(index)]),
            "InstUebSEPA" => self.content_result_data([inst_ueb_sepa_response_root(index)]),
            "LastB2BSEPA" => self.content_result_data([last_b2b_sepa_response_root(index)]),
            "LastCOR1SEPA" => self.content_result_data([last_cor1_sepa_response_root(index)]),
            "LastSEPA" => self.content_result_data([last_sepa_response_root(index)]),
            "MultiLastB2BSEPA" => {
                self.content_result_data([multi_last_b2b_sepa_response_root(index)])
            }
            "MultiLastCOR1SEPA" => {
                self.content_result_data([multi_last_cor1_sepa_response_root(index)])
            }
            "MultiLastSEPA" => self.content_result_data([multi_last_sepa_response_root(index)]),
            "Kontoauszug" => self.content_result_data([kontoauszug_response_root(index)]),
            "KontoauszugPdf" => self.content_result_data([kontoauszug_pdf_response_root(index)]),
            "TermUeb" => self.content_result_data([term_ueb_response_root(index)]),
            "TermUebEdit" => self.content_result_data([term_ueb_edit_response_root(index)]),
            "TermUebSEPA" => self.content_result_data([term_ueb_sepa_response_root(index)]),
            "TermMultiUebSEPA" => {
                self.content_result_data([term_multi_ueb_sepa_response_root(index)])
            }
            "TermUebSEPAEdit" => {
                self.content_result_data([term_ueb_sepa_edit_response_root(index)])
            }
            "TermUebList" => self.content_result_data([term_ueb_list_response_root(index)]),
            "TermUebSEPAList" => {
                self.content_result_data([term_ueb_sepa_list_response_root(index)])
            }
            "KUmsAll" => self.content_result_data([kums_response_root("KUmsZeitRes7", index)]),
            "KUmsZeitSEPA" => {
                self.content_result_data([kums_response_root("KUmsZeitSEPARes7", index)])
            }
            "KUmsAllCamt" => {
                self.content_result_data([kums_response_root("KUmsZeitCamtRes1", index)])
            }
            "KUmsNew" => self.content_result_data([kums_response_root("KUmsNewRes7", index)]),
            "SEPAInfo" => self.content_result_data([sepa_info_response_root(index)]),
            "SaldoReq" => self.content_result_data([saldo_response_root(index)]),
            "SaldoReqAll" => self.content_result_data(
                counted_prefixes(&self.values, "CustomMsgRes.GVRes")
                    .into_iter()
                    .map(|prefix| format!("{prefix}.SaldoRes7")),
            ),
            "Status" => self.content_result_data(self.status_response_roots()),
            "TANList" => self.content_result_data(self.tan_list_response_roots()),
            "TANMediaList" => self.content_result_data([tan_media_list_response_root(index)]),
            "VoP" => self.content_result_data([vop_response_root(index)]),
            "WPDepotList" => self.content_result_data([wp_depot_list_response_root(index)]),
            "WPDepotUms" => self.content_result_data([wp_depot_ums_response_root(index)]),
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

    fn acc_info_result_for_root(&self, root: String) -> Option<GvrAccInfo> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        acc_info_entry_from_values(&self.values, &root).map(|entry| GvrAccInfo {
            entries: vec![entry],
        })
    }

    fn card_list_result_for_root(&self, root: String) -> Option<GvrCardList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrCardList {
            entries: vec![card_info_from_values(&self.values, &root)],
        })
    }

    fn dauer_list_result_for_root(&self, root: String) -> Option<GvrDauerList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrDauerList {
            entries: vec![dauer_list_entry_from_values(&self.values, &root)],
        })
    }

    fn dauer_last_sepa_list_result_for_root(&self, root: String) -> Option<GvrDauerList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrDauerList {
            entries: vec![dauer_last_sepa_list_entry_from_values(&self.values, &root)],
        })
    }

    fn dauer_new_result_for_root(&self, root: String) -> Option<GvrDauerNew> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrDauerNew {
            order_id: optional_value(&self.values, &format!("{root}.orderid")),
        })
    }

    fn info_list_result_for_root(&self, root: String) -> Option<GvrInfoList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        let entries = counted_prefixes(&self.values, &format!("{root}.InfoInfo"))
            .into_iter()
            .filter_map(|prefix| info_list_info_from_values(&self.values, &prefix))
            .collect();

        Some(GvrInfoList { entries })
    }

    fn info_order_result_for_root(&self, root: String) -> Option<GvrInfoOrder> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        let entries = counted_prefixes(&self.values, &format!("{root}.Info"))
            .into_iter()
            .filter_map(|prefix| info_order_info_from_values(&self.values, &prefix))
            .collect();

        Some(GvrInfoOrder { entries })
    }

    fn fest_cond_list_result_for_root(&self, root: String) -> Option<GvrFestCondList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        let entries = counted_prefixes(&self.values, &format!("{root}.FestCond"))
            .into_iter()
            .filter_map(|prefix| fest_cond_from_values(&self.values, &root, &prefix))
            .collect();

        Some(GvrFestCondList { entries })
    }

    fn fest_list_result_for_root(&self, root: String) -> Option<GvrFestList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrFestList {
            entries: vec![fest_list_entry_from_values(&self.values, &root)],
        })
    }

    fn kontoauszug_result_for_root(&self, root: String) -> Option<GvrKontoauszug> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrKontoauszug {
            entries: vec![kontoauszug_entry_from_values(&self.values, &root)],
        })
    }

    fn kontoauszug_pdf_result_for_root(&self, root: String) -> Option<GvrKontoauszug> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrKontoauszug {
            entries: vec![kontoauszug_pdf_entry_from_values(&self.values, &root)],
        })
    }

    fn status_response_roots(&self) -> Vec<String> {
        counted_prefixes(&self.values, "CustomMsgRes.GVRes")
            .into_iter()
            .map(|prefix| format!("{prefix}.StatusRes4"))
            .filter(|root| self.values.contains_key(&format!("{root}.SegHead.code")))
            .collect()
    }

    fn status_result(&self) -> Option<GvrStatus> {
        let entries = self
            .status_response_roots()
            .into_iter()
            .filter_map(|root| status_entry_from_values(&self.values, &root))
            .collect::<Vec<_>>();

        (!entries.is_empty()).then_some(GvrStatus { entries })
    }

    fn tan_list_response_roots(&self) -> Vec<String> {
        counted_prefixes(&self.values, "CustomMsgRes.GVRes")
            .into_iter()
            .map(|prefix| format!("{prefix}.TANListListRes1"))
            .filter(|root| self.values.contains_key(&format!("{root}.SegHead.code")))
            .collect()
    }

    fn tan_list_result(&self) -> Option<GvrTanList> {
        let lists = self
            .tan_list_response_roots()
            .into_iter()
            .filter_map(|root| tan_list_entry_from_values(&self.values, &root))
            .collect::<Vec<_>>();

        (!lists.is_empty()).then_some(GvrTanList { lists })
    }

    fn dauer_edit_result_for_root(&self, root: String) -> Option<GvrDauerEdit> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrDauerEdit {
            order_id: optional_value(&self.values, &format!("{root}.orderid")),
            order_id_old: optional_value(&self.values, &format!("{root}.orderidold")),
        })
    }

    fn term_ueb_result_for_root(&self, root: String) -> Option<GvrTermUeb> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrTermUeb {
            order_id: optional_value(&self.values, &format!("{root}.orderid")),
        })
    }

    fn inst_ueb_sepa_result_for_root(&self, root: String) -> Option<GvrInstUebSepa> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrInstUebSepa {
            order_id: optional_value(&self.values, &format!("{root}.orderid")),
            order_status: optional_value(&self.values, &format!("{root}.orderstatus")),
            cancellation_code: optional_value(&self.values, &format!("{root}.ccode")),
        })
    }

    fn last_sepa_result_for_root(&self, root: String) -> Option<GvrLastSepa> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrLastSepa {
            order_id: optional_value(&self.values, &format!("{root}.orderid")),
        })
    }

    fn term_ueb_edit_result_for_root(&self, root: String) -> Option<GvrTermUebEdit> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrTermUebEdit {
            order_id: optional_value(&self.values, &format!("{root}.orderid")),
            order_id_old: optional_value(&self.values, &format!("{root}.orderidold")),
        })
    }

    fn term_ueb_list_result_for_root(&self, root: String) -> Option<GvrTermUebList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrTermUebList {
            entries: vec![term_ueb_list_entry_from_values(&self.values, &root)],
        })
    }

    fn vop_result_for_root(&self, root: String) -> Option<GvrVoP> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        let report_desc = optional_value(&self.values, &format!("{root}.reportdesc"));
        let report = optional_value(&self.values, &format!("{root}.report"));
        let mut result = VoPResult {
            vop_id: optional_value(&self.values, &format!("{root}.vopid")),
            polling_id: optional_value(&self.values, &format!("{root}.pollingid")),
            text: optional_value(&self.values, &format!("{root}.infotext")),
            items: Vec::new(),
        };

        if report_desc.is_none() || report.is_none() {
            result.items.push(vop_result_item_from_values(
                &self.values,
                &format!("{root}.result"),
            ));
        }

        Some(GvrVoP {
            result: Some(result),
        })
    }

    fn wp_depot_list_result_for_root(&self, root: String) -> Option<GvrWPDepotList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        let data_535 = optional_value(&self.values, &format!("{root}.data535"))
            .map(|data| vec![decode_umlauts(&data)])
            .unwrap_or_default();

        Some(GvrWPDepotList {
            data_535,
            rest: None,
        })
    }

    fn wp_depot_ums_result_for_root(&self, root: String) -> Option<GvrWPDepotUms> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        let data_536 = optional_value(&self.values, &format!("{root}.data536"))
            .map(|data| vec![decode_umlauts(&data)])
            .unwrap_or_default();

        Some(GvrWPDepotUms {
            data_536,
            entries: Vec::new(),
            rest: None,
        })
    }

    fn kums_result_for_root(&self, root: String) -> Option<HbciJobResultData> {
        let booked = self.values.get(&format!("{root}.booked"));
        let notbooked = self.values.get(&format!("{root}.notbooked"));

        if booked.is_none() && notbooked.is_none() {
            return None;
        }

        let mut result = GvrKUms::new();
        if let Some(booked) = booked {
            result.append_mt940_data(decode_umlauts(booked));
        }
        if let Some(notbooked) = notbooked {
            result.append_mt942_data(decode_umlauts(notbooked));
        }

        Some(HbciJobResultData::KUms(result))
    }

    fn tan_media_list_result_for_root(&self, root: String) -> Option<GvrTanMediaList> {
        self.values.get(&format!("{root}.SegHead.code"))?;

        let media = counted_prefixes(&self.values, &format!("{root}.MediaInfo"))
            .into_iter()
            .filter_map(|prefix| tan_media_info_from_values(&self.values, &prefix))
            .collect();

        Some(GvrTanMediaList {
            tan_option: optional_i32(&self.values, &format!("{root}.tanoption")),
            media,
        })
    }

    fn kums_all_camt_result_for_root(&self, root: String) -> Option<HbciJobResultData> {
        let booked_messages = counted_value_keys(&self.values, &format!("{root}.booked.message"));
        let notbooked = self.values.get(&format!("{root}.notbooked"));

        if booked_messages.is_empty() && notbooked.is_none() {
            return None;
        }

        let mut result = GvrKUms::new();
        for key in booked_messages {
            if let Some(booked) = self.values.get(&key) {
                result.camt_booked.push(booked.clone());
            }
        }
        if let Some(notbooked) = notbooked {
            result.camt_not_booked.push(notbooked.clone());
        }

        Some(HbciJobResultData::KUms(result))
    }
}

fn vop_result_item_from_values(values: &BTreeMap<String, String>, prefix: &str) -> VoPResultItem {
    let status = optional_value(values, &format!("{prefix}.result"))
        .and_then(|code| VoPStatus::from_code(&code));

    VoPResultItem {
        status,
        original: None,
        name: optional_value(values, &format!("{prefix}.differentname")),
        iban: optional_value(values, &format!("{prefix}.iban")),
        usage: None,
        amount: None,
        text: optional_value(values, &format!("{prefix}.reason")),
    }
}

fn tan_media_info_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrTanMediaInfo> {
    Some(GvrTanMediaInfo {
        media_category: optional_value(values, &format!("{prefix}.mediacategory")),
        status: optional_value(values, &format!("{prefix}.status")),
        card_number: optional_value(values, &format!("{prefix}.cardnumber")),
        card_seq_number: optional_value(values, &format!("{prefix}.cardseqnumber")),
        card_type: optional_i32(values, &format!("{prefix}.cardtype")),
        valid_from: optional_value(values, &format!("{prefix}.validfrom")),
        valid_to: optional_value(values, &format!("{prefix}.validto")),
        tan_list_number: optional_value(values, &format!("{prefix}.tanlistnumber")),
        media_name: optional_value(values, &format!("{prefix}.medianame")),
        mobile_number: optional_value(values, &format!("{prefix}.mobilenumber")),
        mobile_number_secure: optional_value(values, &format!("{prefix}.mobilenumber_secure")),
        free_tans: optional_i32(values, &format!("{prefix}.freetans")),
        last_use: optional_value(values, &format!("{prefix}.lastuse")),
        activated_on: optional_value(values, &format!("{prefix}.activatedon")),
    })
    .filter(|info| info.media_category.is_some())
}

fn info_list_info_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrInfoListInfo> {
    let code = optional_value(values, &format!("{prefix}.code"))?;
    let comments = counted_value_keys(values, &format!("{prefix}.comment"))
        .into_iter()
        .filter_map(|key| optional_value(values, &key))
        .collect();

    Some(GvrInfoListInfo {
        code: Some(code),
        description: optional_value(values, &format!("{prefix}.descr")),
        info_type: optional_value(values, &format!("{prefix}.type")),
        format: optional_value(values, &format!("{prefix}.format")),
        date: optional_value(values, &format!("{prefix}.version")),
        comments,
    })
}

fn info_order_info_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrInfoOrderInfo> {
    let code = optional_value(values, &format!("{prefix}.code"))?;

    Some(GvrInfoOrderInfo {
        code: Some(code),
        message: optional_value(values, &format!("{prefix}.msg")),
    })
}

fn fest_cond_from_values(
    values: &BTreeMap<String, String>,
    root: &str,
    prefix: &str,
) -> Option<GvrFestCond> {
    let anlagedatum = optional_value(values, &format!("{prefix}.anlagedate"))?;

    Some(GvrFestCond {
        anlagedatum: Some(anlagedatum),
        ablaufdatum: optional_value(values, &format!("{prefix}.ablaufdate")),
        zinssatz: optional_value(values, &format!("{prefix}.zinssatz"))
            .and_then(|value| wrt_to_thousand_scaled_i64(&value)),
        zinsmethode: optional_value(values, &format!("{prefix}.zinsmethode"))
            .and_then(|value| fest_cond_method(&value)),
        minbetrag: value_from_values(values, &format!("{prefix}.MinBetrag")),
        maxbetrag: value_from_values(values, &format!("{prefix}.MaxBetrag"))
            .filter(|value| !value.value.is_empty() || value.curr.is_some()),
        id: optional_value(values, &format!("{prefix}.condid")),
        name: optional_value(values, &format!("{prefix}.condbez")),
        version: optional_value(values, &format!("{root}.FestCondVersion.version")),
        date: optional_value(values, &format!("{root}.FestCondVersion.date")),
        time: optional_value(values, &format!("{root}.FestCondVersion.time")),
    })
}

fn fest_list_entry_from_values(values: &BTreeMap<String, String>, root: &str) -> GvrFestListEntry {
    GvrFestListEntry {
        anlagekonto: national_account_from_values(values, &format!("{root}.Anlagekto")),
        belastungskonto: national_account_from_values(values, &format!("{root}.Belastungskto")),
        ausbuchungskonto: national_account_from_values(values, &format!("{root}.Ausbuchungskto")),
        zinskonto: national_account_from_values(values, &format!("{root}.Zinskto")),
        id: optional_value(values, &format!("{root}.kontakt")),
        anlagebetrag: value_from_values(values, &format!("{root}.Anlagebetrag")),
        zinsbetrag: value_from_values(values, &format!("{root}.Zinsbetrag")),
        konditionen: fest_cond_from_values(values, root, &format!("{root}.FestCond")),
        verlaengern: optional_value(values, &format!("{root}.wiederanlage"))
            .is_some_and(|value| value == "2"),
        kontoauszug: optional_i32(values, &format!("{root}.kontoauszug")).unwrap_or_default(),
        status: optional_i32(values, &format!("{root}.status")).unwrap_or_default(),
        verlaengerung: fest_list_prolong_from_values(values, &format!("{root}.Prolong")),
    }
}

fn fest_list_prolong_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrFestListProlong> {
    let laufzeit = optional_i32(values, &format!("{prefix}.laufzeit"))?;

    Some(GvrFestListProlong {
        laufzeit,
        betrag: value_from_values(values, &format!("{prefix}.BTG")),
        verlaengern: optional_value(values, &format!("{prefix}.wiederanlage"))
            .is_some_and(|value| value == "2"),
    })
}

fn kontoauszug_pdf_entry_from_values(
    values: &BTreeMap<String, String>,
    root: &str,
) -> GvrKontoauszugEntry {
    GvrKontoauszugEntry {
        format: Some(KontoauszugFormat::Pdf),
        data: optional_value(values, &format!("{root}.booked"))
            .and_then(|value| kontoauszug_pdf_data_bytes(&value)),
        date: optional_value(values, &format!("{root}.date")),
        start_date: optional_value(values, &format!("{root}.TimeRange.startdate")),
        end_date: optional_value(values, &format!("{root}.TimeRange.enddate")),
        year: optional_i32(values, &format!("{root}.year")),
        number: optional_i32(values, &format!("{root}.number")),
        iban: optional_value(values, &format!("{root}.iban")),
        bic: optional_value(values, &format!("{root}.bic")),
        name: optional_value(values, &format!("{root}.name")),
        name2: optional_value(values, &format!("{root}.name2")),
        name3: optional_value(values, &format!("{root}.name3")),
        filename: optional_value(values, &format!("{root}.filename")),
        receipt: optional_value(values, &format!("{root}.receipt"))
            .map(|value| latin1_lossy_bytes(&value)),
        ..GvrKontoauszugEntry::default()
    }
}

fn kontoauszug_entry_from_values(
    values: &BTreeMap<String, String>,
    root: &str,
) -> GvrKontoauszugEntry {
    let format = optional_value(values, &format!("{root}.format"))
        .and_then(|value| KontoauszugFormat::from_code(&value));

    GvrKontoauszugEntry {
        format,
        data: optional_value(values, &format!("{root}.booked")).map(|value| {
            let decoded = if format == Some(KontoauszugFormat::Mt940) {
                decode_umlauts(&value)
            } else {
                value
            };
            latin1_lossy_bytes(&decoded)
        }),
        date: optional_value(values, &format!("{root}.date")),
        start_date: optional_value(values, &format!("{root}.TimeRange.startdate")),
        end_date: optional_value(values, &format!("{root}.TimeRange.enddate")),
        year: optional_i32(values, &format!("{root}.year")),
        number: optional_i32(values, &format!("{root}.number")),
        abschluss_info: optional_value(values, &format!("{root}.abschlussinfo")),
        kunden_info: optional_value(values, &format!("{root}.kondinfo")),
        werbetext: optional_value(values, &format!("{root}.ads")),
        iban: optional_value(values, &format!("{root}.iban")),
        bic: optional_value(values, &format!("{root}.bic")),
        name: optional_value(values, &format!("{root}.name")),
        name2: optional_value(values, &format!("{root}.name2")),
        name3: optional_value(values, &format!("{root}.name3")),
        receipt: optional_value(values, &format!("{root}.receipt"))
            .map(|value| latin1_lossy_bytes(&value)),
        ..GvrKontoauszugEntry::default()
    }
}

fn kontoauszug_pdf_data_bytes(value: &str) -> Option<Vec<u8>> {
    if value.starts_with("%PDF-") {
        Some(latin1_lossy_bytes(value))
    } else {
        STANDARD.decode(value.as_bytes()).ok()
    }
}

fn latin1_lossy_bytes(value: &str) -> Vec<u8> {
    value
        .chars()
        .map(|character| {
            let code = character as u32;
            if code <= 0xff { code as u8 } else { b'?' }
        })
        .collect()
}

fn fest_cond_method(value: &str) -> Option<i32> {
    match value {
        "A" => Some(GvrFestCond::METHOD_30_360),
        "B" => Some(GvrFestCond::METHOD_2831_360),
        "C" => Some(GvrFestCond::METHOD_2831_365366),
        "D" => Some(GvrFestCond::METHOD_30_365366),
        "E" => Some(GvrFestCond::METHOD_2831_365),
        "F" => Some(GvrFestCond::METHOD_30_365),
        _ => None,
    }
}

fn wrt_to_thousand_scaled_i64(value: &str) -> Option<i64> {
    let compact = value.trim().replace(',', ".");
    let (negative, unsigned) = if let Some(stripped) = compact.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = compact.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, compact.as_str())
    };
    let mut parts = unsigned.split('.');
    let integer = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || (integer.is_empty() && fraction.is_empty())
        || !integer.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let integer = if integer.is_empty() {
        0_i64
    } else {
        integer.parse::<i64>().ok()?
    };
    let mut fraction_scaled = 0_i64;
    let mut factor = 100_i64;
    for ch in fraction.chars().take(3) {
        fraction_scaled += ch.to_digit(10)? as i64 * factor;
        factor /= 10;
    }

    let scaled = integer.checked_mul(1000)?.checked_add(fraction_scaled)?;
    Some(if negative { -scaled } else { scaled })
}

fn status_entry_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrStatusEntry> {
    values.get(&format!("{prefix}.SegHead.code"))?;
    let segment_ref = optional_value(values, &format!("{prefix}.segref"));
    let mut return_value = collect_return_values(values, prefix, ReturnValueScope::Segment)
        .into_iter()
        .next();
    if let Some(value) = return_value.as_mut() {
        value.segment_ref = segment_ref.clone();
        value.element = None;
    }

    Some(GvrStatusEntry {
        dialog_id: optional_value(values, &format!("{prefix}.MsgRef.dialogid")),
        msg_num: optional_value(values, &format!("{prefix}.MsgRef.msgnum")),
        segment_ref,
        date: optional_value(values, &format!("{prefix}.date")),
        time: optional_value(values, &format!("{prefix}.time")),
        return_value,
    })
}

fn tan_list_entry_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrTanListEntry> {
    values.get(&format!("{prefix}.SegHead.code"))?;
    let tan_infos = counted_prefixes(values, &format!("{prefix}.TANInfo"))
        .into_iter()
        .filter_map(|prefix| tan_info_from_values(values, &prefix))
        .collect();

    Some(GvrTanListEntry {
        status: optional_value(values, &format!("{prefix}.liststatus")),
        number: optional_value(values, &format!("{prefix}.listnumber")),
        date: optional_value(values, &format!("{prefix}.date")),
        tan_count: optional_i32(values, &format!("{prefix}.noftansperlist")),
        used_tan_count: optional_i32(values, &format!("{prefix}.nofusedtansperlist")),
        tan_infos,
    })
}

fn tan_info_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<GvrTanInfo> {
    let usage_code = optional_i32(values, &format!("{prefix}.usagecode"))?;

    Some(GvrTanInfo {
        usage_code: Some(usage_code),
        usage_text: optional_value(values, &format!("{prefix}.usagetxt")),
        tan: optional_value(values, &format!("{prefix}.tan")),
        usage_date: optional_value(values, &format!("{prefix}.usagedate")),
        usage_time: optional_value(values, &format!("{prefix}.usagetime")),
    })
}

fn card_info_from_values(values: &BTreeMap<String, String>, prefix: &str) -> GvrCardInfo {
    GvrCardInfo {
        card_type: optional_i32(values, &format!("{prefix}.cardtype")),
        card_number: optional_value(values, &format!("{prefix}.cardnumber")),
        card_order_number: optional_value(values, &format!("{prefix}.nextcardnumber")),
        owner: optional_value(values, &format!("{prefix}.name")),
        valid_from: optional_value(values, &format!("{prefix}.validfrom")),
        valid_until: optional_value(values, &format!("{prefix}.validuntil")),
        limit: value_from_values(values, &format!("{prefix}.cardlimit")),
        comment: optional_value(values, &format!("{prefix}.comment")),
    }
}

fn acc_info_entry_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrAccInfoEntry> {
    let number = optional_value(values, &format!("{prefix}.My.number"))?;

    Some(GvrAccInfoEntry {
        account: Konto {
            country: optional_value(values, &format!("{prefix}.My.KIK.country")),
            blz: optional_value(values, &format!("{prefix}.My.KIK.blz")),
            number: Some(number),
            subnumber: optional_value(values, &format!("{prefix}.My.subnumber")),
            name: optional_value(values, &format!("{prefix}.name")),
            name2: optional_value(values, &format!("{prefix}.name2")),
            acctype: optional_value(values, &format!("{prefix}.acctype")),
            account_type: optional_value(values, &format!("{prefix}.accbez")),
            curr: optional_value(values, &format!("{prefix}.curr")),
            ..Konto::default()
        },
        account_kind: optional_i32(values, &format!("{prefix}.acctype")),
        created: optional_value(values, &format!("{prefix}.opendate")),
        sollzins: optional_value(values, &format!("{prefix}.sollzins")),
        habenzins: optional_value(values, &format!("{prefix}.habenzins")),
        ueberzins: optional_value(values, &format!("{prefix}.overdrivezins")),
        kredit: value_from_values(values, &format!("{prefix}.kredit")),
        ref_account: national_account_from_values(values, &format!("{prefix}.refkto")),
        versandart: optional_i32(values, &format!("{prefix}.versandart")),
        turnus: optional_i32(values, &format!("{prefix}.turnus")),
        comment: optional_value(values, &format!("{prefix}.info")),
        address: acc_info_address_from_values(values, &format!("{prefix}.Address")),
    })
}

fn national_account_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<Konto> {
    let number = optional_value(values, &format!("{prefix}.number"))?;

    Some(Konto {
        country: optional_value(values, &format!("{prefix}.KIK.country")),
        blz: optional_value(values, &format!("{prefix}.KIK.blz")),
        number: Some(number),
        subnumber: optional_value(values, &format!("{prefix}.subnumber")),
        curr: None,
        ..Konto::default()
    })
}

fn acc_info_address_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrAccInfoAddress> {
    optional_value(values, &format!("{prefix}.name1"))?;

    Some(GvrAccInfoAddress {
        name1: optional_value(values, &format!("{prefix}.name1")),
        name2: optional_value(values, &format!("{prefix}.name2")),
        street_pf: optional_value(values, &format!("{prefix}.street_pf")),
        plz_ort: optional_value(values, &format!("{prefix}.plz_ort")),
        plz: optional_value(values, &format!("{prefix}.plz")),
        ort: optional_value(values, &format!("{prefix}.ort")),
        country: optional_value(values, &format!("{prefix}.country")),
        tel: optional_value(values, &format!("{prefix}.tel")),
        fax: optional_value(values, &format!("{prefix}.fax")),
        email: optional_value(values, &format!("{prefix}.email")),
    })
}

fn dauer_list_entry_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> GvrDauerListEntry {
    let sepapain_raw = optional_value(values, &format!("{prefix}.sepapain"));
    let pain_transfer = sepapain_raw
        .as_deref()
        .and_then(|pain| parse_pain_001_transfers(pain).ok())
        .and_then(|transfers| transfers.into_iter().next());
    let classic_other = classic_inland_other_account_from_values(values, prefix);

    GvrDauerListEntry {
        my: ktv_int_account_from_values(values, &format!("{prefix}.My")),
        other: pain_transfer
            .as_ref()
            .map(|transfer| transfer.destination.clone())
            .unwrap_or(classic_other),
        value: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.value.clone())
            .or_else(|| value_from_values(values, &format!("{prefix}.BTG"))),
        key: optional_value(values, &format!("{prefix}.key")),
        addkey: optional_value(values, &format!("{prefix}.addkey")),
        usage: pain_transfer
            .as_ref()
            .map(|transfer| transfer.usage.clone())
            .unwrap_or_else(|| classic_inland_usage_from_values(values, prefix)),
        nextdate: optional_value(values, &format!("{prefix}.date")),
        orderid: optional_value(values, &format!("{prefix}.orderid")),
        firstdate: optional_value(values, &format!("{prefix}.DauerDetails.firstdate")),
        timeunit: optional_value(values, &format!("{prefix}.DauerDetails.timeunit")),
        turnus: optional_i32(values, &format!("{prefix}.DauerDetails.turnus")),
        execday: optional_i32(values, &format!("{prefix}.DauerDetails.execday")),
        exectime: optional_value(values, &format!("{prefix}.DauerDetails.exectime")),
        lastdate: optional_value(values, &format!("{prefix}.DauerDetails.lastdate")),
        aussetzung: dauer_list_aussetzung_from_values(values, &format!("{prefix}.Aussetzung")),
        can_change: optional_jn(values, &format!("{prefix}.canchange")).unwrap_or(true),
        can_skip: optional_jn(values, &format!("{prefix}.canskip")).unwrap_or(true),
        can_delete: optional_jn(values, &format!("{prefix}.candel")).unwrap_or(true),
        pmtinfid: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.payment_info_id.clone()),
        purposecode: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.purpose_code.clone()),
        debit_type: None,
        sequence_type: None,
        creditor_id: None,
        mandate_id: None,
        mandate_date_of_signature: None,
        end_to_end_id: None,
        sepadescr: optional_value(values, &format!("{prefix}.sepadescr")),
        sepapain_raw,
    }
}

fn dauer_last_sepa_list_entry_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> GvrDauerListEntry {
    let sepapain_raw = optional_value(values, &format!("{prefix}.sepapain"));
    let direct_debit = sepapain_raw
        .as_deref()
        .and_then(|pain| parse_pain_008_direct_debits(pain).ok())
        .and_then(|direct_debits| direct_debits.into_iter().next());

    GvrDauerListEntry {
        my: ktv_int_account_from_values(values, &format!("{prefix}.My")),
        other: direct_debit
            .as_ref()
            .map(|direct_debit| direct_debit.debtor.clone())
            .unwrap_or_default(),
        value: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.value.clone()),
        key: None,
        addkey: None,
        usage: direct_debit
            .as_ref()
            .map(|direct_debit| direct_debit.usage.clone())
            .unwrap_or_default(),
        nextdate: None,
        orderid: optional_value(values, &format!("{prefix}.orderid")),
        firstdate: optional_value(values, &format!("{prefix}.DauerDetails.firstdate")),
        timeunit: optional_value(values, &format!("{prefix}.DauerDetails.timeunit")),
        turnus: optional_i32(values, &format!("{prefix}.DauerDetails.turnus")),
        execday: optional_i32(values, &format!("{prefix}.DauerDetails.execday")),
        exectime: optional_value(values, &format!("{prefix}.DauerDetails.exectime")),
        lastdate: optional_value(values, &format!("{prefix}.DauerDetails.lastdate")),
        aussetzung: dauer_list_aussetzung_from_values(values, &format!("{prefix}.Aussetzung")),
        can_change: optional_jn(values, &format!("{prefix}.canchange")).unwrap_or(true),
        can_skip: optional_jn(values, &format!("{prefix}.canskip")).unwrap_or(true),
        can_delete: optional_jn(values, &format!("{prefix}.candel")).unwrap_or(true),
        pmtinfid: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.payment_info_id.clone()),
        purposecode: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.purpose_code.clone()),
        debit_type: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.debit_type.clone()),
        sequence_type: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.sequence_type.clone()),
        creditor_id: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.creditor_id.clone()),
        mandate_id: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.mandate_id.clone()),
        mandate_date_of_signature: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.mandate_date_of_signature.clone()),
        end_to_end_id: direct_debit
            .as_ref()
            .and_then(|direct_debit| direct_debit.end_to_end_id.clone()),
        sepadescr: optional_value(values, &format!("{prefix}.sepadescr")),
        sepapain_raw,
    }
}

fn classic_inland_other_account_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Konto {
    let mut other = ktv_int_account_from_values(values, &format!("{prefix}.Other"));
    other.name = optional_value(values, &format!("{prefix}.name"));
    other.name2 = optional_value(values, &format!("{prefix}.name2"));
    other
}

fn classic_inland_usage_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Vec<String> {
    counted_value_keys(values, &format!("{prefix}.usage.usage"))
        .into_iter()
        .filter_map(|key| optional_value(values, &key))
        .collect()
}

fn term_ueb_list_entry_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> GvrTermUebListEntry {
    let sepapain_raw = optional_value(values, &format!("{prefix}.sepapain"));
    let pain_transfer = sepapain_raw
        .as_deref()
        .and_then(|pain| parse_pain_001_transfers(pain).ok())
        .and_then(|transfers| transfers.into_iter().next());
    let classic_other = classic_inland_other_account_from_values(values, prefix);

    GvrTermUebListEntry {
        my: ktv_int_account_from_values(values, &format!("{prefix}.My")),
        other: pain_transfer
            .as_ref()
            .map(|transfer| transfer.destination.clone())
            .unwrap_or(classic_other),
        value: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.value.clone())
            .or_else(|| value_from_values(values, &format!("{prefix}.BTG"))),
        key: optional_value(values, &format!("{prefix}.key")),
        addkey: optional_value(values, &format!("{prefix}.addkey")),
        usage: pain_transfer
            .as_ref()
            .map(|transfer| transfer.usage.clone())
            .unwrap_or_else(|| classic_inland_usage_from_values(values, prefix)),
        date: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.execution_date.clone())
            .or_else(|| optional_value(values, &format!("{prefix}.date"))),
        orderid: optional_value(values, &format!("{prefix}.orderid"))
            .or_else(|| optional_value(values, &format!("{prefix}.id"))),
        can_change: optional_jn(values, &format!("{prefix}.canchange")).unwrap_or(true),
        can_delete: optional_jn(values, &format!("{prefix}.candel")).unwrap_or(true),
        sepadescr: optional_value(values, &format!("{prefix}.sepadescr")),
        sepapain_raw,
    }
}

fn ktv_int_account_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Konto {
    Konto {
        iban: optional_value(values, &format!("{prefix}.iban")),
        bic: optional_value(values, &format!("{prefix}.bic")),
        number: optional_value(values, &format!("{prefix}.number")),
        subnumber: optional_value(values, &format!("{prefix}.subnumber")),
        country: optional_value(values, &format!("{prefix}.KIK.country")),
        blz: optional_value(values, &format!("{prefix}.KIK.blz")),
        ..Konto::default()
    }
}

fn dauer_list_aussetzung_from_values(
    values: &BTreeMap<String, String>,
    prefix: &str,
) -> Option<GvrDauerListAussetzung> {
    Some(GvrDauerListAussetzung {
        annual: optional_jn(values, &format!("{prefix}.annual"))?,
        startdate: optional_value(values, &format!("{prefix}.startdate")),
        enddate: optional_value(values, &format!("{prefix}.enddate")),
        number: optional_value(values, &format!("{prefix}.number")),
        newvalue: value_from_values(values, &format!("{prefix}.newvalue")),
    })
}

fn update_passport_tan_media_names_from_results(
    passport: &mut PinTanPassport,
    results: &[HbciJobResult],
) {
    for result in results {
        let Some(HbciJobResultData::TanMediaList(data)) = result.result.as_ref() else {
            continue;
        };
        let names = data.active_media_names();
        if !names.is_empty() {
            passport.set_tan_media_names(names);
        }
    }
}

fn update_passport_job_persistent_data_from_results(
    passport: &mut PinTanPassport,
    jobs: &[HbciJob],
    results: &[HbciJobResult],
) {
    for (job, result) in jobs.iter().zip(results) {
        match result.job_name.as_str() {
            "DauerList" | "DauerSEPAList" | "DauerLastSEPAList" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot = dauer_persistent_snapshot(&result.result_data);
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "DauerSEPAEdit" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_sepa_request_persistent_snapshot(job, passport, "DauerSEPAEdit1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "DauerSEPADel" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_sepa_request_persistent_snapshot(job, passport, "DauerSEPADel1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "DauerSEPANew" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_sepa_request_persistent_snapshot(job, passport, "DauerSEPANew1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "DauerLastSEPANew" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_last_sepa_request_persistent_snapshot(job, passport, "DauerLastSEPANew1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "DauerNew" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    classic_inland_request_persistent_snapshot(job, passport, "DauerNew5");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "DauerEdit" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot = classic_inland_request_persistent_snapshot_without_orderid(
                    job,
                    passport,
                    "DauerEdit5",
                );
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("dauer_{order_id}"), snapshot);
                }
            }
            "TermUeb" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    classic_inland_request_persistent_snapshot(job, passport, "TermUeb4");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termueb_{order_id}"), snapshot);
                }
            }
            "TermUebEdit" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot = classic_inland_request_persistent_snapshot_without_id(
                    job,
                    passport,
                    "TermUebEdit4",
                );
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termueb_{order_id}"), snapshot);
                }
            }
            "TermUebSEPA" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_sepa_request_persistent_snapshot(job, passport, "TermUebSEPA1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termueb_{order_id}"), snapshot);
                }
            }
            "TermMultiUebSEPA" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_sepa_request_persistent_snapshot(job, passport, "TermSammelUebSEPA1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termueb_{order_id}"), snapshot);
                }
            }
            "TermUebSEPAEdit" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot =
                    dauer_sepa_request_persistent_snapshot(job, passport, "TermUebSEPAEdit1");
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termueb_{order_id}"), snapshot);
                }
            }
            "LastB2BSEPA" | "LastCOR1SEPA" | "LastSEPA" | "MultiLastB2BSEPA"
            | "MultiLastCOR1SEPA" | "MultiLastSEPA" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let lowlevel_segment = match result.job_name.as_str() {
                    "LastB2BSEPA" => "LastB2BSEPA1",
                    "LastCOR1SEPA" => "LastCOR1SEPA1",
                    "MultiLastB2BSEPA" => "SammelLastB2BSEPA1",
                    "MultiLastCOR1SEPA" => "SammelLastCOR1SEPA1",
                    "MultiLastSEPA" => "SammelLastSEPA1",
                    _ => "LastSEPA1",
                };
                let snapshot =
                    last_sepa_request_persistent_snapshot(job, passport, lowlevel_segment);
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termlast_{order_id}"), snapshot);
                }
            }
            "TermUebList" | "TermUebSEPAList" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
                    .or_else(|| result.result_data.get("content.id"))
                    .filter(|order_id| !order_id.is_empty())
                else {
                    continue;
                };
                let snapshot = dauer_persistent_snapshot(&result.result_data);
                if !snapshot.is_empty() {
                    passport.set_persistent_data(format!("termueb_{order_id}"), snapshot);
                }
            }
            _ => {}
        }
    }
}

fn dauer_persistent_snapshot(result_data: &BTreeMap<String, String>) -> Properties {
    let mut snapshot = Properties::new();
    let prefix = "content.";

    for (key, value) in result_data {
        let Some(suffix) = key.strip_prefix(prefix) else {
            continue;
        };
        if suffix.starts_with("SegHead.")
            || suffix == "orderid"
            || suffix.ends_with(".orderid")
            || suffix == "id"
            || suffix.ends_with(".id")
        {
            continue;
        }
        snapshot.insert(suffix.to_owned(), value.clone());
    }

    snapshot
}

fn dauer_sepa_request_persistent_snapshot(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Properties {
    let mut snapshot = Properties::new();
    let prefix = format!("{lowlevel_segment}.");

    for (key, value) in job.lowlevel_params() {
        let Some(suffix) = key.strip_prefix(&prefix) else {
            continue;
        };
        if suffix.starts_with("sepa.") {
            continue;
        }
        snapshot.insert(
            suffix.to_owned(),
            snapshot_value_for_request_suffix(suffix, value),
        );
    }

    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    insert_optional_snapshot_value(&mut snapshot, "My.bic", account.bic.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.iban", account.iban.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.KIK.country", account.country.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.KIK.blz", account.blz.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.number", account.number.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.subnumber", account.subnumber.as_deref());

    if !snapshot.contains_key("sepadescr") {
        snapshot.insert("sepadescr".to_owned(), PAIN_001_001_02_URN.to_owned());
    }
    if !snapshot.contains_key("sepapain")
        && let Some(sepapain) = job_param(job, &format!("{lowlevel_segment}.sepapain"), "_sepapain")
    {
        snapshot.insert("sepapain".to_owned(), sepa_binary_value(sepapain));
    }

    for (suffix, frontend) in [
        ("DauerDetails.firstdate", "firstdate"),
        ("DauerDetails.timeunit", "timeunit"),
        ("DauerDetails.turnus", "turnus"),
        ("DauerDetails.execday", "execday"),
        ("DauerDetails.lastdate", "lastdate"),
    ] {
        if !snapshot.contains_key(suffix)
            && let Some(value) = job_param(job, &format!("{lowlevel_segment}.{suffix}"), frontend)
        {
            snapshot.insert(suffix.to_owned(), value.to_owned());
        }
    }

    snapshot
}

fn dauer_last_sepa_request_persistent_snapshot(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Properties {
    let mut snapshot = last_sepa_request_persistent_snapshot(job, passport, lowlevel_segment);

    for (suffix, frontend) in [
        ("DauerDetails.firstdate", "firstdate"),
        ("DauerDetails.timeunit", "timeunit"),
        ("DauerDetails.turnus", "turnus"),
        ("DauerDetails.execday", "execday"),
        ("DauerDetails.lastdate", "lastdate"),
    ] {
        if !snapshot.contains_key(suffix)
            && let Some(value) = job_param(job, &format!("{lowlevel_segment}.{suffix}"), frontend)
        {
            snapshot.insert(suffix.to_owned(), value.to_owned());
        }
    }

    snapshot
}

fn last_sepa_request_persistent_snapshot(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Properties {
    let mut snapshot = Properties::new();
    let prefix = format!("{lowlevel_segment}.");

    for (key, value) in job.lowlevel_params() {
        let Some(suffix) = key.strip_prefix(&prefix) else {
            continue;
        };
        snapshot.insert(
            suffix.to_owned(),
            snapshot_value_for_request_suffix(suffix, value),
        );
    }

    let account = standing_order_sepa_account(job, passport, lowlevel_segment);
    insert_optional_snapshot_value(&mut snapshot, "My.bic", account.bic.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.iban", account.iban.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.KIK.country", account.country.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.KIK.blz", account.blz.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.number", account.number.as_deref());
    insert_optional_snapshot_value(&mut snapshot, "My.subnumber", account.subnumber.as_deref());

    if !snapshot.contains_key("sepadescr") {
        snapshot.insert("sepadescr".to_owned(), PAIN_008_001_01_URN.to_owned());
    }
    if !snapshot.contains_key("sepapain")
        && let Some(sepapain) = job_param(job, &format!("{lowlevel_segment}.sepapain"), "_sepapain")
    {
        snapshot.insert("sepapain".to_owned(), sepa_binary_value(sepapain));
    }

    snapshot
}

fn classic_inland_request_persistent_snapshot(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Properties {
    let mut snapshot = Properties::new();
    let prefix = format!("{lowlevel_segment}.");

    for (key, value) in job.lowlevel_params() {
        let Some(suffix) = key.strip_prefix(&prefix) else {
            continue;
        };
        snapshot.insert(suffix.to_owned(), value.clone());
    }

    let source_account = classic_national_job_account(
        job,
        passport.first_account().cloned(),
        lowlevel_segment,
        "My",
        "src",
    );
    insert_classic_account_snapshot(&mut snapshot, "My", &source_account);

    let destination_account =
        classic_national_job_account(job, None, lowlevel_segment, "Other", "dst");
    insert_classic_account_snapshot(&mut snapshot, "Other", &destination_account);

    snapshot
}

fn classic_inland_request_persistent_snapshot_without_id(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Properties {
    let mut snapshot = classic_inland_request_persistent_snapshot(job, passport, lowlevel_segment);
    snapshot.retain(|key, _value| key != "id" && !key.ends_with(".id"));
    snapshot
}

fn classic_inland_request_persistent_snapshot_without_orderid(
    job: &HbciJob,
    passport: &PinTanPassport,
    lowlevel_segment: &str,
) -> Properties {
    let mut snapshot = classic_inland_request_persistent_snapshot(job, passport, lowlevel_segment);
    snapshot.retain(|key, _value| key != "orderid" && !key.ends_with(".orderid"));
    snapshot
}

fn insert_classic_account_snapshot(snapshot: &mut Properties, prefix: &str, account: &Konto) {
    insert_optional_snapshot_value(
        snapshot,
        &format!("{prefix}.number"),
        account.number.as_deref(),
    );
    insert_optional_snapshot_value(
        snapshot,
        &format!("{prefix}.subnumber"),
        account.subnumber.as_deref(),
    );
    insert_optional_snapshot_value(
        snapshot,
        &format!("{prefix}.KIK.country"),
        account.country.as_deref().or(Some("DE")),
    );
    insert_optional_snapshot_value(
        snapshot,
        &format!("{prefix}.KIK.blz"),
        account.blz.as_deref(),
    );
}

fn snapshot_value_for_request_suffix(suffix: &str, value: &str) -> String {
    if suffix == "sepapain" {
        sepa_binary_value(value)
    } else {
        value.to_owned()
    }
}

fn insert_optional_snapshot_value(snapshot: &mut Properties, key: &str, value: Option<&str>) {
    if snapshot.contains_key(key) {
        return;
    }
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        snapshot.insert(key.to_owned(), value.to_owned());
    }
}

fn update_passport_accounts_from_sepa_info(
    passport: &mut PinTanPassport,
    jobs: &[HbciJob],
    response_status: &ParsedResponseStatus,
) {
    for (index, job) in jobs.iter().enumerate() {
        if job.name() == "SEPAInfo" {
            passport.update_accounts_from_sepa_info_values(
                &response_status.values,
                &sepa_info_response_root(index),
            );
        }
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
        creditorid: None,
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

fn optional_i32(values: &BTreeMap<String, String>, key: &str) -> Option<i32> {
    optional_value(values, key).and_then(|value| value.parse().ok())
}

fn optional_jn(values: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    optional_value(values, key).map(|value| value == "J")
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
    index + 3
}
