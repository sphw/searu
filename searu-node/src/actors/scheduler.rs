use crate::{
    storage::{Event, Storage},
    types::{Node, Vm},
};

use super::Actor;

pub struct Scheduler {
    storage: Storage,
}

impl Scheduler {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl Actor for Scheduler {
    type Message = Event<Vm>;

    type Response = ();

    async fn handle(
        &mut self,
        message: Self::Message,
    ) -> Result<Self::Response, crate::types::Error> {
        match message {
            Event::New(mut vm) | Event::Update { new: mut vm, .. } => {
                if vm.status.node.is_none() {
                    let nodes: Vec<Node> = self.storage.list().await?;
                    let node = &nodes[0];
                    vm.status.node = Some(node.metadata.name.clone());
                    self.storage.store(&vm).await?;
                }
            }
            Event::Delete(_) => {}
        }
        Ok(())
    }
}
