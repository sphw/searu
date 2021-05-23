use std::{collections::HashSet, net::Ipv4Addr, num::Wrapping};

use crate::{
    storage::{Event, Storage},
    types::{Node, Vm, Vpc},
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
    type Message = Events;

    type Response = ();

    async fn handle(
        &mut self,
        message: Self::Message,
    ) -> Result<Self::Response, crate::types::Error> {
        match message {
            Events::VmEvent(message) => match message {
                Event::New(mut vm) | Event::Update { new: mut vm, .. } => {
                    if vm.status.node.is_none() {
                        let nodes: Vec<Node> = self.storage.list().await?;
                        let node = &nodes[0];
                        vm.status.node = Some(node.metadata.name.clone());
                        self.storage.store(&vm).await?;
                    }
                }
                Event::Delete(_) => {}
            },
            Events::VpcEvent(message) => match message {
                Event::New(mut vpc) | Event::Update { new: mut vpc, .. } => {
                    if vpc.spec.multicast_ip.is_none() {
                        let mut used_ips: HashSet<Ipv4Addr> = HashSet::default();
                        let vpcs: Vec<Vpc> = self.storage.list().await?;
                        let mut largest_octet = Wrapping(0);
                        for vpc in &vpcs {
                            if let Some(ip) = vpc.spec.multicast_ip {
                                used_ips.insert(ip);
                                let octet = Wrapping(ip.octets()[3]);
                                if largest_octet < octet {
                                    largest_octet = octet;
                                }
                            }
                        }
                        let mut attempts: u16 = 0;
                        largest_octet += Wrapping(1);
                        let mut ip = Ipv4Addr::from([239, 1, 1, largest_octet.0]);
                        while used_ips.contains(&ip) && attempts < 512 {
                            attempts += 1;
                            largest_octet += Wrapping(1);
                            ip = Ipv4Addr::from([239, 1, 1, largest_octet.0]);
                        }
                        if attempts >= 512 {
                            // TODO: Handle failure to schedule
                            return Ok(());
                        }
                        vpc.spec.multicast_ip = Some(ip);
                        self.storage.store(&vpc).await?;
                    }
                    if vpc.spec.vni.is_none() {
                        let mut used_vnis: HashSet<u16> = HashSet::default();
                        let vpcs: Vec<Vpc> = self.storage.list().await?;
                        let mut largest_vni = Wrapping(0);
                        for vpc in &vpcs {
                            if let Some(vni) = vpc.spec.vni {
                                used_vnis.insert(vni);
                                if largest_vni.0 < vni {
                                    largest_vni = Wrapping(vni);
                                }
                            }
                        }
                        let mut attempts: u16 = 0;
                        largest_vni += Wrapping(1);
                        while used_vnis.contains(&largest_vni.0) && attempts < 1024 {
                            attempts += 1;
                            largest_vni += Wrapping(1);
                        }
                        if attempts >= 512 {
                            // TODO: Handle failure to schedule
                            return Ok(());
                        }
                        vpc.spec.vni = Some(largest_vni.0);
                        self.storage.store(&vpc).await?;
                    }
                }
                Event::Delete(_) => {}
            },
        }

        Ok(())
    }
}

pub enum Events {
    VmEvent(Event<Vm>),
    VpcEvent(Event<Vpc>),
}
