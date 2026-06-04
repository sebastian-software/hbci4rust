use crate::comm::{CommClient, CommRequest, DefaultCommClient};
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv::{HbciJob, JobRegistry};
use crate::gv_result::{HbciExecStatus, HbciJobResult};
use crate::passport::PinTanPassport;
use crate::protocol::{HbciMessage, load_protocol_spec};

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

        let results = self
            .queue
            .drain(..)
            .map(|job| HbciJobResult {
                job_name: job.name().to_owned(),
                success: response.status < 400,
                raw_response: Some(String::from_utf8_lossy(&response.body).into_owned()),
            })
            .collect();

        Ok(HbciExecStatus {
            success: response.status < 400,
            job_results: results,
            messages: Vec::new(),
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
