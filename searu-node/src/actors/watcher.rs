use super::{Handle, Scheduler, VmSupervisor};
use crate::{storage::Storage, types::Vm};
use futures::StreamExt;
use tokio::task::JoinHandle;

pub struct VmWatcher {
    storage: Storage,
    scheduler: Handle<Scheduler>,
    supervisor: Handle<VmSupervisor>,
}

impl VmWatcher {
    pub fn new(
        storage: Storage,
        scheduler: Handle<Scheduler>,
        supervisor: Handle<VmSupervisor>,
    ) -> Self {
        Self {
            storage,
            scheduler,
            supervisor,
        }
    }

    pub fn spawn(self) -> JoinHandle<Result<(), anyhow::Error>> {
        tokio::spawn(async move {
            let mut stream = self.storage.watch::<Vm>().await?;
            while let Some(event) = stream.next().await {
                let _ = self.scheduler.send(event.clone()).await;
                let _ = self.supervisor.send(event).await;
            }
            Ok(())
        })
    }
}
