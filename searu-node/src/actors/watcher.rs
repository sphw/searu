use super::{Events, Handle, Scheduler, VmSupervisor, VpcSupervisor};
use crate::{
    storage::Storage,
    types::{Vm, Vpc},
};
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
                let _ = self.scheduler.send(Events::VmEvent(event.clone())).await;
                if let Err(err) = self.supervisor.send(event).await {
                    println!("error: {:?}", err);
                }
            }
            Ok(())
        })
    }
}

pub struct VpcWatcher {
    storage: Storage,
    scheduler: Handle<Scheduler>,
    supervisor: Handle<VpcSupervisor>,
}

impl VpcWatcher {
    pub fn new(
        storage: Storage,
        scheduler: Handle<Scheduler>,
        supervisor: Handle<VpcSupervisor>,
    ) -> Self {
        Self {
            storage,
            scheduler,
            supervisor,
        }
    }

    pub fn spawn(self) -> JoinHandle<Result<(), anyhow::Error>> {
        tokio::spawn(async move {
            let mut stream = self.storage.watch::<Vpc>().await?;
            while let Some(event) = stream.next().await {
                let _ = self.scheduler.send(Events::VpcEvent(event.clone())).await;
                println!("sending");
                if let Err(err) = self.supervisor.send(event).await {
                    println!("error: {:?}", err);
                }
            }
            Ok(())
        })
    }
}
