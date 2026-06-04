use crate::comm::{CommClient, CommRequest, DefaultCommClient};
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv::{HbciJob, JobRegistry};
use crate::gv_result::{HbciExecStatus, HbciJobResult};
use crate::passport::PinTanPassport;

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

        let body = self
            .queue
            .iter()
            .map(HbciJob::name)
            .collect::<Vec<_>>()
            .join(",");

        let response = self
            .comm
            .send(CommRequest::new(host, body.into_bytes()))
            .await?;

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
}
