use std::{net::Ipv4Addr, process::Stdio};

use tokio::process::{Child, Command};

use crate::{
    storage::Event,
    types::{Error, Vm},
};

use super::Actor;

pub struct DHCPActor {
    range: (Ipv4Addr, Ipv4Addr),
    vpc_name: String,
    nat_gateway: Option<Ipv4Addr>,
    dnsmasq: Option<Child>,
    netmask: Ipv4Addr,
}

#[async_trait::async_trait]
impl Actor for DHCPActor {
    type Message = Event<Vm>;

    type Response = ();

    async fn handle(
        &mut self,
        message: Self::Message,
    ) -> Result<Self::Response, crate::types::Error> {
        todo!()
    }

    async fn init(&mut self) -> Result<(), Error> {
        self.spawn_dhcpd()?;
        Ok(())
    }
}
impl DHCPActor {
    pub fn new(
        range: (Ipv4Addr, Ipv4Addr),
        vpc_name: String,
        nat_gateway: Option<Ipv4Addr>,
        netmask: Ipv4Addr,
    ) -> Self {
        Self {
            range,
            vpc_name,
            nat_gateway,
            dnsmasq: None,
            netmask,
        }
    }

    fn spawn_dhcpd(&mut self) -> Result<(), Error> {
        self.dnsmasq = None; // Kill current dnsmasq;
        let mut args = vec![
            "--log-facility=-".to_string(),
            "-k".to_string(),
            "--bind-dynamic".to_string(),
            "-C".to_string(),
            "/dev/null".to_string(),
            format!("--interface=b{}", self.vpc_name),
            "--port=0".to_string(),
            format!(
                "--dhcp-range={},{},{},12h",
                self.range.0, self.range.1, self.netmask
            ),
            "--dhcp-option=6,8.8.8.8".to_string(),
        ];
        if let Some(nat_gateway) = self.nat_gateway {
            args.push(format!("--dhcp-option=3,{}", nat_gateway));
        }
        println!("spawning dnsmasq");
        let child = Command::new("dnsmasq")
            .kill_on_drop(true)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()?;
        self.dnsmasq = Some(child);
        Ok(())
    }
}
