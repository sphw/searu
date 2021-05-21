#![allow(clippy::upper_case_acronyms)]

use etcd_client::{KeyValue, ResponseHeader};
use ipnet::Ipv4Net;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;

mod auth;

pub use auth::*;

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub name: String,
}

impl Object for Project {
    const OBJECT_TYPE: &'static str = "project";

    fn metadata(&self) -> Cow<'_, Metadata> {
        Cow::Owned(Metadata {
            name: self.name.to_string(),
            ..Default::default()
        })
    }

    fn set_version(&mut self, rev: i64) {}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Vm {
    pub metadata: Metadata,
    pub spec: VmSpec,
    #[serde(default)]
    pub status: VmStatus,
}

impl Object for Vm {
    const OBJECT_TYPE: &'static str = "vm";

    fn metadata(&self) -> Cow<'_, Metadata> {
        Cow::Borrowed(&self.metadata)
    }

    fn set_version(&mut self, rev: i64) {
        self.metadata.version = Some(rev)
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct VmSpec {
    pub vpc: String,
    pub cpus: u8,
    pub memory: usize,
}

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct VmStatus {
    pub node: Option<String>,
    pub state: VmState,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum VmState {
    Uncreated,
    PoweredOff,
    PoweredOn,
}

impl Default for VmState {
    fn default() -> Self {
        VmState::Uncreated
    }
}

#[derive(Serialize, Deserialize)]
pub struct Vpc {
    metadata: Metadata,
    spec: VpcSpec,
}

#[derive(Serialize, Deserialize)]
pub struct VpcSpec {
    subnet: Ipv4Net,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Metadata {
    pub name: String,
    pub project: String,
    pub version: Option<i64>,
}

pub trait Object: Serialize + DeserializeOwned {
    const OBJECT_TYPE: &'static str;

    fn metadata(&self) -> Cow<'_, Metadata>;

    fn key(&self) -> String {
        format!("{}/{}", Self::OBJECT_TYPE, self.metadata().name)
    }

    fn set_version(&mut self, rev: i64);

    fn parse(kv: &KeyValue) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let mut obj: Self = serde_json::from_slice(kv.value())?;
        obj.set_version(kv.version());
        Ok(obj)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Node {
    pub metadata: Metadata,
    pub cpu_count: usize,
    pub cpu_freq: u64,
    pub memory: u64,
}

impl Object for Node {
    const OBJECT_TYPE: &'static str = "node";

    fn metadata(&self) -> Cow<'_, Metadata> {
        Cow::Borrowed(&self.metadata)
    }

    fn set_version(&mut self, rev: i64) {
        self.metadata.version = Some(rev);
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("etcd: {0}")]
    Etcd(#[from] etcd_client::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::error::Error),
    #[error("bcrypt: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("unauthorized")]
    Unauthorized,
    #[error("jwt: {0}")]
    JWT(#[from] jsonwebtoken::errors::Error),
    #[error("oneshot recv error: {0}")]
    Oneshot(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("actor failed to send")]
    ActorSend,
    #[error("sysinfo: {0}")]
    SysInfo(#[from] sys_info::Error),
    #[error("hypervisor: {0}")]
    Hypervisor(#[from] hypervisor::HypervisorError),
    #[error("vmm: {0}")]
    Vmm(String),
    #[error("io: {0}")]
    IO(#[from] std::io::Error),
    #[error("join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("http error: {0}")]
    Http(#[from] hyper::http::Error),
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<vmm::Error> for Error {
    fn from(err: vmm::Error) -> Self {
        Error::Vmm(err.to_string())
    }
}

impl From<vmm::api::ApiError> for Error {
    fn from(err: vmm::api::ApiError) -> Self {
        Error::Vmm(format!("{:?}", err))
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    msg: String,
}

impl<'r> rocket::response::Responder<'r, 'static> for Error {
    fn respond_to(self, _request: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        use rocket::{
            http::{ContentType, Status},
            Response,
        };
        use std::io::Cursor;

        let msg = self.to_string();
        let resp = ErrorResponse { msg };
        let resp = serde_json::to_string(&resp).map_err(|_| Status::InternalServerError)?;
        Response::build()
            .header(ContentType::new("application", "json"))
            .sized_body(resp.len(), Cursor::new(resp))
            .ok()
    }
}

#[derive(Serialize)]
pub struct ListResponse<T> {
    pub objects: Vec<T>,
    pub next_page: String,
}
