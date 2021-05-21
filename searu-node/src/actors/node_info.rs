use crate::{storage::Storage, types::Node};

use super::Actor;

pub struct NodeInfo {
    storage: Storage,
}

impl NodeInfo {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl Actor for NodeInfo {
    type Message = ();

    type Response = ();

    async fn handle(
        &mut self,
        _message: Self::Message,
    ) -> Result<Self::Response, crate::types::Error> {
        let hostname = sys_info::hostname()?;
        let memory = sys_info::mem_info()?;
        let node = Node {
            metadata: crate::types::Metadata {
                name: hostname,
                ..Default::default()
            },
            cpu_count: sys_info::cpu_num()? as usize,
            cpu_freq: sys_info::cpu_speed()?,
            memory: memory.total,
        };
        self.storage.store(&node).await?;
        Ok(())
    }
}
