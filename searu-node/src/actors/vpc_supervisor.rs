use std::net::IpAddr;

use super::Actor;
use crate::{
    storage::{Event, Storage},
    types::{Error, Vpc},
};
use futures::stream::TryStreamExt;
use netlink_packet_route::rtnl::link::LinkMessage;
use rtnetlink::Handle;

pub struct VpcSupervisor {
    _storage: Storage,
    handle: Handle,
}

impl VpcSupervisor {
    pub fn new(_storage: Storage, handle: Handle) -> Self {
        Self { _storage, handle }
    }
}

#[async_trait::async_trait]
impl Actor for VpcSupervisor {
    type Message = Event<Vpc>;

    type Response = ();

    async fn handle(
        &mut self,
        message: Self::Message,
    ) -> Result<Self::Response, crate::types::Error> {
        match message {
            Event::New(vpc) | Event::Update { new: vpc, .. } => {
                if let Some(multicast_ip) = vpc.spec.multicast_ip {
                    if let Some(vni) = vpc.spec.vni {
                        // let mut links = self
                        //     .handle
                        //     .link()
                        //     .get()
                        //     .set_name_filter("")
                        //     .execute();
                        //if let Some(link) = links.try_next().await? {
                        self.handle
                            .link()
                            .add()
                            .vxlan(format!("vx{}", vpc.metadata.name), vni as u32) //TODO: Add VNI scheduling
                            .link(4) //TODO: Use name filterings
                            .group(multicast_ip)
                            .port(0)
                            .up()
                            .execute()
                            .await?;
                        let bridge_name = format!("b{}", vpc.metadata.name);
                        // let veth_name = format!("veth{}", vpc.metadata.name);
                        // let veth_p_name = format!("veth{}p", vpc.metadata.name);
                        self.handle
                            .link()
                            .add()
                            .bridge(bridge_name.clone())
                            .execute()
                            .await?;
                        // self.handle
                        //     .link()
                        //     .add()
                        //     .veth(veth_name.clone(), veth_p_name.clone())
                        //     .execute()
                        //     .await?;

                        let bridge = self.handle.get_link_by_name(bridge_name).await?;
                        // let veth_p = self.get_link_by_name(veth_p_name).await?;
                        // let veth = self.get_link_by_name(veth_name).await?;
                        // self.handle
                        //     .link()
                        //     .set(veth_p.header.index)
                        //     .master(bridge.header.index)
                        //     .execute()
                        //     .await?;
                        // self.handle
                        //     .link()
                        //     .set(veth_p.header.index)
                        //     .up()
                        //     .execute()
                        //     .await?;
                        // self.handle
                        //     .link()
                        //     .set(veth.header.index)
                        //     .up()
                        //     .execute()
                        //     .await?;
                        self.handle
                            .link()
                            .set(bridge.header.index)
                            .up()
                            .execute()
                            .await?;

                        // TODO: Remoe this in favour of a DHCP solution
                        let host_ip = vpc
                            .spec
                            .subnet
                            .hosts()
                            .next()
                            .ok_or_else(|| Error::NotFound("host ip".to_string()))?;
                        self.handle
                            .address()
                            .add(bridge.header.index, IpAddr::V4(host_ip), 24)
                            .execute()
                            .await?;
                        self.handle
                            .link()
                            .set(bridge.header.index)
                            .up()
                            .execute()
                            .await?;
                    }
                }
            }
            Event::Delete(vpc) => {
                let vx = self.handle.get_link_by_name(format!("vx{}", vpc)).await?;
                self.handle.link().del(vx.header.index).execute().await?;
                let b = self.handle.get_link_by_name(format!("b{}", vpc)).await?;
                self.handle.link().del(b.header.index).execute().await?;
                let veth = self.handle.get_link_by_name(format!("veth{}", vpc)).await?;
                self.handle.link().del(veth.header.index).execute().await?;
            }
        }
        Ok(())
    }
}

#[async_trait::async_trait]
pub trait HandleExt {
    async fn get_link_by_name(&self, name: String) -> Result<LinkMessage, Error>;
}

#[async_trait::async_trait]
impl HandleExt for Handle {
    async fn get_link_by_name(&self, name: String) -> Result<LinkMessage, Error> {
        self.link()
            .get()
            .set_name_filter(name.clone())
            .execute()
            .try_next()
            .await?
            .ok_or_else(|| Error::NotFound(format!("link: {}", name)))
    }
}
