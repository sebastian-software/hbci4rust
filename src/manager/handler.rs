use std::collections::BTreeMap;
use std::str;

use crate::callback::{CallbackDataType, CallbackEvent, CallbackReason, HbciCallback};
use crate::comm::{CommClient, CommRequest, CommResponse, DefaultCommClient};
use crate::dialog::DialogContext;
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv::{HbciJob, JobRegistry};
use crate::gv_result::{
    GvrAccInfo, GvrAccInfoAddress, GvrAccInfoEntry, GvrDauerEdit, GvrDauerList,
    GvrDauerListAussetzung, GvrDauerListEntry, GvrDauerNew, GvrInfoList, GvrInfoListInfo,
    GvrInstUebSepa, GvrKUms, GvrSaldoReq, GvrSaldoReqInfo, GvrTanMediaInfo, GvrTanMediaList,
    GvrTermUeb, GvrTermUebEdit, GvrTermUebList, GvrTermUebListEntry, HbciDialogStatus,
    HbciExecStatus, HbciInstMessage, HbciJobResult, HbciJobResultData, HbciMsgStatus,
    HbciReturnValue, HbciStatus, Konto, Saldo, Value,
};
use crate::passport::{
    ONESTEP_TAN_METHOD_ID, PinTanPassport, TanMethodOption, TanMethodSelection, UserSig,
};
use crate::protocol::{HbciMessage, load_protocol_spec, parse_wire_message};
use crate::sepa::{CAMT_052_001_01_URN, PAIN_001_001_02_URN, parse_pain_001_transfers};
use crate::swift::decode_umlauts;
use crate::tools::Properties;

use super::{
    ChallengeInfo, OrderHashMode, PinTanSignatureContext, apply_pintan_sig_head,
    apply_pintan_sig_tail_from_head, apply_pintan_user_sig_to_sig_tail,
    collect_pintan_segment_codes, collect_pintan_signature_range,
};

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
        job.verify_constraints()?;
        self.queue.push(job);
        Ok(())
    }

    pub fn try_add_to_queue_with_initial_tan_job(&mut self, mut job: HbciJob) -> HbciResult<()> {
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
        job.verify_constraints()?;
        let callback = super::callback();
        job.verify_account_checks(callback.as_deref()).await?;
        self.queue.push(job);
        Ok(())
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
        "DauerSEPADel" => render_dauer_sepa_del(message, job, index, passport),
        "DauerSEPAEdit" => render_dauer_sepa_edit(message, job, index, passport),
        "DauerSEPAList" => render_dauer_sepa_list(message, job, index, passport),
        "DauerSEPANew" => render_dauer_sepa_new(message, job, index, passport),
        "InfoList" => render_info_list(message, job, index),
        "InstUebSEPA" => render_inst_ueb_sepa(message, job, index, passport),
        "KUmsAll" => render_kums_all(message, job, index, passport),
        "KUmsAllCamt" => render_kums_all_camt(message, job, index, passport),
        "KUmsNew" => render_kums_new(message, job, index, passport),
        "KUmsZeitSEPA" => render_kums_zeit_sepa(message, job, index, passport),
        "Receipt" => render_receipt(message, job, index),
        "SEPAInfo" => render_sepa_info(message, index),
        "SaldoReq" => render_saldo_request(message, job, index, passport),
        "SaldoReqAll" => render_saldo_request_all(message, job, index, passport),
        "TANMediaList" => render_tan_media_list(message, job, index),
        "TAN2Step" => render_tan2step(message, job, index),
        "TermUebSEPA" => render_term_ueb_sepa(message, job, index, passport),
        "TermUebSEPADel" => render_term_ueb_sepa_del(message, job, index, passport),
        "TermUebSEPAEdit" => render_term_ueb_sepa_edit(message, job, index, passport),
        "TermUebSEPAList" => render_term_ueb_sepa_list(message, job, index, passport),
        "UebSEPA" => render_ueb_sepa(message, job, index, passport),
        "UmbSEPA" => render_umb_sepa(message, job, index, passport),
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
    let account = effective_job_account(task, passport, task_info.lowlevel_segment, "my");
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
        "InstUebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKIPZ",
            lowlevel_segment: "InstUebSEPA1",
            path: "CustomMsg.GV.InstUebSEPA1",
        }),
        "InfoList" => Ok(OrderhashSourceJobInfo {
            code: "HKKIA",
            lowlevel_segment: "InfoList4",
            path: "CustomMsg.GV.InfoList4",
        }),
        "UebSEPA" => Ok(OrderhashSourceJobInfo {
            code: "HKCCS",
            lowlevel_segment: "UebSEPA1",
            path: "CustomMsg.GV.UebSEPA1",
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

fn info_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.InfoListRes4".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.InfoListRes4", index + 1)
    }
}

fn dauer_sepa_list_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerSEPAListRes2".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerSEPAListRes2", index + 1)
    }
}

fn dauer_sepa_edit_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerSEPAEditRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerSEPAEditRes1", index + 1)
    }
}

fn dauer_sepa_new_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.DauerSEPANewRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.DauerSEPANewRes1", index + 1)
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

fn inst_ueb_sepa_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.InstUebSEPARes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.InstUebSEPARes1", index + 1)
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

fn sepa_info_response_root(index: usize) -> String {
    if index == 0 {
        "CustomMsgRes.GVRes.SEPAInfoRes1".to_owned()
    } else {
        format!("CustomMsgRes.GVRes_{}.SEPAInfoRes1", index + 1)
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
            include_order_id: true,
            include_date: true,
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct DauerSepaOrderRenderSpec {
    job_name: &'static str,
    lowlevel_segment: &'static str,
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
        .unwrap_or(PAIN_001_001_02_URN),
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
            "DauerSEPADel" => self
                .dauer_edit_result_for_root(dauer_sepa_edit_response_root(index))
                .map(HbciJobResultData::DauerEdit),
            "DauerSEPAEdit" => self
                .dauer_edit_result_for_root(dauer_sepa_edit_response_root(index))
                .map(HbciJobResultData::DauerEdit),
            "DauerSEPAList" => self
                .dauer_list_result_for_root(dauer_sepa_list_response_root(index))
                .map(HbciJobResultData::DauerList),
            "DauerSEPANew" => self
                .dauer_new_result_for_root(dauer_sepa_new_response_root(index))
                .map(HbciJobResultData::DauerNew),
            "InfoList" => self
                .info_list_result_for_root(info_list_response_root(index))
                .map(HbciJobResultData::InfoList),
            "InstUebSEPA" => self
                .inst_ueb_sepa_result_for_root(inst_ueb_sepa_response_root(index))
                .map(HbciJobResultData::InstUebSepa),
            "TermUebSEPA" => self
                .term_ueb_result_for_root(term_ueb_sepa_response_root(index))
                .map(HbciJobResultData::TermUeb),
            "TermUebSEPAEdit" => self
                .term_ueb_edit_result_for_root(term_ueb_sepa_edit_response_root(index))
                .map(HbciJobResultData::TermUebEdit),
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
            "TANMediaList" => self
                .tan_media_list_result_for_root(tan_media_list_response_root(index))
                .map(HbciJobResultData::TanMediaList),
            _ => None,
        }
    }

    fn result_data_for_job(&self, job: &HbciJob, index: usize) -> BTreeMap<String, String> {
        match job.name() {
            "AccInfo" => self.content_result_data([acc_info_response_root(index)]),
            "DauerSEPADel" => self.content_result_data([dauer_sepa_edit_response_root(index)]),
            "DauerSEPAEdit" => self.content_result_data([dauer_sepa_edit_response_root(index)]),
            "DauerSEPAList" => self.content_result_data([dauer_sepa_list_response_root(index)]),
            "DauerSEPANew" => self.content_result_data([dauer_sepa_new_response_root(index)]),
            "InfoList" => self.content_result_data([info_list_response_root(index)]),
            "InstUebSEPA" => self.content_result_data([inst_ueb_sepa_response_root(index)]),
            "TermUebSEPA" => self.content_result_data([term_ueb_sepa_response_root(index)]),
            "TermUebSEPAEdit" => {
                self.content_result_data([term_ueb_sepa_edit_response_root(index)])
            }
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
            "TANMediaList" => self.content_result_data([tan_media_list_response_root(index)]),
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

    fn dauer_list_result_for_root(&self, root: String) -> Option<GvrDauerList> {
        self.values.get(&format!("{root}.SegHead.code"))?;
        Some(GvrDauerList {
            entries: vec![dauer_list_entry_from_values(&self.values, &root)],
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

    GvrDauerListEntry {
        my: ktv_int_account_from_values(values, &format!("{prefix}.My")),
        other: pain_transfer
            .as_ref()
            .map(|transfer| transfer.destination.clone())
            .unwrap_or_default(),
        value: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.value.clone()),
        key: None,
        addkey: None,
        usage: pain_transfer
            .as_ref()
            .map(|transfer| transfer.usage.clone())
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
        pmtinfid: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.payment_info_id.clone()),
        purposecode: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.purpose_code.clone()),
        sepadescr: optional_value(values, &format!("{prefix}.sepadescr")),
        sepapain_raw,
    }
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

    GvrTermUebListEntry {
        my: ktv_int_account_from_values(values, &format!("{prefix}.My")),
        other: pain_transfer
            .as_ref()
            .map(|transfer| transfer.destination.clone())
            .unwrap_or_default(),
        value: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.value.clone()),
        key: None,
        addkey: None,
        usage: pain_transfer
            .as_ref()
            .map(|transfer| transfer.usage.clone())
            .unwrap_or_default(),
        date: pain_transfer
            .as_ref()
            .and_then(|transfer| transfer.execution_date.clone()),
        orderid: optional_value(values, &format!("{prefix}.orderid")),
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
            "DauerSEPAList" => {
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
            "TermUebSEPAList" => {
                let Some(order_id) = result
                    .result_data
                    .get("content.orderid")
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
        if suffix.starts_with("SegHead.") || suffix == "orderid" || suffix.ends_with(".orderid") {
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
