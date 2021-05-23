use std::{
    collections::HashMap, path::PathBuf, process::Stdio, sync::Arc, thread::JoinHandle,
    time::Duration,
};

use tokio::{process::Command, sync::Mutex};

use crate::vmm::{
    CmdlineConfig, ConsoleConfig, ConsoleOutputMode, CpusConfig, DiskConfig, KernelConfig,
    MemoryConfig, RngConfig, VmConfig,
};
use crate::{
    storage::{Event, Storage},
    types::{Error, Vm, VmState, VmStatus},
};
use hyper::Body;
use hyperlocal::{UnixClientExt, Uri};
use rand::{distributions::Alphanumeric, Rng};
use vmm_sys_util::eventfd::EventFd;

use super::Actor;

pub struct VmSupervisor {
    storage: Storage,
    node_name: String,
    vms: HashMap<String, VmInstance>,
}

impl VmSupervisor {
    pub fn new(storage: Storage) -> Result<Self, Error> {
        Ok(Self {
            storage,
            node_name: sys_info::hostname()?,
            vms: HashMap::default(),
        })
    }
}

#[async_trait::async_trait]
impl Actor for VmSupervisor {
    type Message = Event<Vm>;

    type Response = ();

    async fn handle(
        &mut self,
        message: Self::Message,
    ) -> Result<Self::Response, crate::types::Error> {
        println!("{:?}", message);
        match message {
            Event::New(mut vm) => {
                if Some(&self.node_name) == vm.status.node.as_ref()
                    && vm.status.state == VmState::Uncreated
                {
                    let name = vm.metadata.name.clone();
                    let inst = VmInstance::new(&vm).await?;
                    self.vms.insert(name, inst);
                    let inst = self.vms.get_mut(&vm.metadata.name).unwrap();
                    vm.status.state = VmState::PoweredOff;
                    self.storage.store(&vm).await?;
                    inst.boot().await?;
                    vm.status.state = VmState::PoweredOn;
                    self.storage.store(&vm).await?;
                }
            }
            Event::Delete(vm) => {
                println!("deleting vm: {:?}", vm);
                let inst = self
                    .vms
                    .remove(&vm)
                    .ok_or_else(|| Error::NotFound(format!("vm: {}", vm)))?;
                println!("shutting down vm");
                inst.shutdown().await?;
            }
            Event::Update { .. } => {}
        }
        Ok(())
    }

    async fn init(&mut self) -> Result<(), Error> {
        let vms: Vec<Vm> = self.storage.list().await?;
        for vm in vms {
            self.handle(Event::New(vm)).await?;
        }
        Ok(())
    }
}

struct VmInstance {
    child: tokio::process::Child,
    client: hyper::Client<hyperlocal::UnixConnector, Body>,
    socket_path: String,
}

impl VmInstance {
    async fn new(vm: &Vm) -> Result<Self, Error> {
        let socket: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let socket_path = format!("/tmp/{}-{}.sock", vm.metadata.name, socket);
        let child = Command::new("./cloud-hypervisor")
            .kill_on_drop(true)
            .args(vec!["--api-socket", &format!("path={}", socket_path)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()?;
        let client = hyper::Client::unix();
        let vm_config = VmConfig {
            cpus: CpusConfig {
                boot_vcpus: vm.spec.cpus,
                max_vcpus: vm.spec.cpus,
                topology: None,
                kvm_hyperv: false,
                max_phys_bits: None,
            },
            memory: MemoryConfig {
                size: 1024 << 20,
                ..Default::default()
            },
            kernel: Some(KernelConfig {
                path: PathBuf::from("./hypervisor-fw"),
            }),
            serial: ConsoleConfig::default_serial(),
            console: ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Pty,
                iommu: false,
            },
            initramfs: None,
            cmdline: CmdlineConfig::default(),
            disks: Some(vec![
                DiskConfig {
                    path: Some(PathBuf::from("./focal-server-cloudimg-amd64.raw")),
                    ..Default::default()
                },
                DiskConfig {
                    path: Some(PathBuf::from("./user-data.img")),
                    ..Default::default()
                },
            ]),
            net: Some(vec![]),
            rng: RngConfig::default(),
            balloon: None,
            fs: None,
            pmem: None,
            devices: None,
            vsock: None,
            iommu: false,
            sgx_epc: None,
            watchdog: false,
            numa: None,
        };
        tokio::time::sleep(Duration::from_millis(500)).await; //TODO: We should have a better way of detecing when the hypervisor is ready
                                                              // but `hyperlocal` appears to panic when it can't access a url
        let body = serde_json::to_string(&vm_config)?;
        let res = client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::PUT)
                    .uri(Uri::new(&socket_path, "/api/v1/vm.create"))
                    .body(Body::from(body))?,
            )
            .await?;
        Ok(Self {
            child,
            client,
            socket_path,
        })
    }

    async fn boot(&self) -> Result<(), Error> {
        let res = self
            .client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::PUT)
                    .uri(Uri::new(&self.socket_path, "/api/v1/vm.boot"))
                    .body(Body::from(""))?,
            )
            .await?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Error> {
        let res = self
            .client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::PUT)
                    .uri(Uri::new(&self.socket_path, "/api/v1/vm.shutdown"))
                    .body(Body::from(""))?,
            )
            .await?;
        Ok(())
    }
}
