use super::HandleExt;
use crate::vmm::{
    CmdlineConfig, ConsoleConfig, ConsoleOutputMode, CpusConfig, DiskConfig, KernelConfig,
    MemoryConfig, NetConfig, RngConfig, VmConfig,
};
use crate::{
    storage::{Event, Storage},
    types::{Error, Vm, VmState},
};
use hyper::Body;
use hyperlocal::{UnixClientExt, Uri};
use rand::{distributions::Alphanumeric, Rng};
use rtnetlink::Handle as NetLinkHandle;
use std::{collections::HashMap, ffi::OsStr, path::PathBuf, process::Stdio, time::Duration};
use tokio::{io::AsyncWriteExt, process::Command};

use super::Actor;

pub struct VmSupervisor {
    storage: Storage,
    node_name: String,
    vms: HashMap<String, VmInstance>,
    netlink_handle: NetLinkHandle,
}

impl VmSupervisor {
    pub fn new(storage: Storage, handle: NetLinkHandle) -> Result<Self, Error> {
        Ok(Self {
            storage,
            node_name: sys_info::hostname()?,
            vms: HashMap::default(),
            netlink_handle: handle,
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
                    && !self.vms.contains_key(&vm.metadata.name)
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
                    let tap = self
                        .netlink_handle
                        .get_link_by_name(format!("ich{}", vm.metadata.name))
                        .await?;
                    let vpc = self
                        .netlink_handle
                        .get_link_by_name(format!("b{}", vm.spec.vpc))
                        .await?;
                    self.netlink_handle
                        .link()
                        .set(tap.header.index)
                        .master(vpc.header.index)
                        .execute()
                        .await?;
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
    _child: tokio::process::Child,
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
        let child = Command::new("./blobs/cloud-hypervisor")
            .kill_on_drop(true)
            .args(vec!["--api-socket", &format!("path={}", socket_path)])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()?;
        let mut disks = vec![DiskConfig {
            path: Some(PathBuf::from("./blobs/focal-server-cloudimg-amd64.raw")),
            ..Default::default()
        }];
        if let Some(ref cloud_init) = vm.spec.cloud_init {
            println!("creating cloud-init");
            let user_data = tempfile::NamedTempFile::new()?;
            let (_, user_data) = user_data.keep()?;
            let mut convert = Command::new("cloud-localds")
                .kill_on_drop(true)
                .args(vec![user_data.as_os_str(), OsStr::new("-")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .stdin(Stdio::piped())
                .spawn()?;
            let stdin = convert.stdin.as_mut().unwrap();
            stdin.write_all(cloud_init.as_bytes()).await?;
            let _ = convert.wait().await?;
            disks.push(DiskConfig {
                path: Some(user_data.to_path_buf()),
                ..Default::default()
            });
            println!("{:?}", user_data);
        }
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
                path: PathBuf::from("./blobs/hypervisor-fw"),
            }),
            serial: ConsoleConfig::default_serial(),
            console: ConsoleConfig {
                file: None,
                mode: ConsoleOutputMode::Pty,
                iommu: false,
            },
            initramfs: None,
            cmdline: CmdlineConfig::default(),
            disks: Some(disks),
            net: Some(vec![NetConfig {
                tap: Some(format!("ich{}", vm.metadata.name)),
                ..Default::default()
            }]),
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
        tokio::time::sleep(Duration::from_millis(500)).await; //TODO: We should have a better way of detecting when the hypervisor is ready
                                                              // but `hyperlocal` appears to panic when it can't access a url
        let body = serde_json::to_string(&vm_config)?;
        let _ = client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::PUT)
                    .uri(Uri::new(&socket_path, "/api/v1/vm.create"))
                    .body(Body::from(body))?,
            )
            .await?;
        Ok(Self {
            _child: child,
            client,
            socket_path,
        })
    }

    async fn boot(&self) -> Result<(), Error> {
        println!("booting vm");
        let _ = self
            .client
            .request(
                hyper::Request::builder()
                    .method(hyper::Method::PUT)
                    .uri(Uri::new(&self.socket_path, "/api/v1/vm.boot"))
                    .body(Body::from(""))?,
            )
            .await?;
        println!("booted vm");
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Error> {
        let _ = self
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
