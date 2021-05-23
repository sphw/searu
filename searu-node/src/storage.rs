use std::sync::Arc;

use etcd_client::{Client, Compare, CompareOp, GetOptions, Txn, TxnOp, WatchOptions};
use futures::{Stream, StreamExt};
use tokio::sync::Mutex;

use crate::types::{Error, Object};

#[derive(Clone)]
pub struct Storage {
    etcd: Arc<Mutex<Client>>,
}

impl Storage {
    pub fn new(etcd: Client) -> Self {
        Self {
            etcd: Arc::new(Mutex::new(etcd)),
        }
    }

    pub async fn store(&self, object: &impl Object) -> Result<(), Error> {
        let key = object.key();
        let mut txn = Txn::new();
        if let Some(version) = object.metadata().version {
            txn = txn.when(vec![Compare::version(
                key.clone(),
                CompareOp::Equal,
                version,
            )]);
        }
        txn = txn.and_then(vec![TxnOp::put(key, serde_json::to_vec(object)?, None)]);
        let mut client = self.etcd.lock().await;
        client.txn(txn).await?;
        Ok(())
    }

    pub async fn get<O: Object>(&self, key: &str) -> Result<Option<O>, Error> {
        let resp = self
            .etcd
            .lock()
            .await
            .get(format!("{}/{}", O::OBJECT_TYPE, key), None)
            .await?;
        if let Some(kv) = resp.kvs().first() {
            O::parse(kv).map(Some)
        } else {
            Ok(None)
        }
    }

    pub async fn delete<O: Object>(&self, key: &str) -> Result<(), Error> {
        let _ = self
            .etcd
            .lock()
            .await
            .delete(format!("{}/{}", O::OBJECT_TYPE, key), None)
            .await?;
        Ok(())
    }

    pub async fn list<O: Object>(&self) -> Result<Vec<O>, Error> {
        let resp = self
            .etcd
            .lock()
            .await
            .get(O::OBJECT_TYPE, Some(GetOptions::default().with_prefix()))
            .await?;
        Ok(resp
            .kvs()
            .iter()
            .filter_map(|kv| O::parse(kv).ok())
            .collect())
    }

    pub async fn watch<O: Object + 'static>(&self) -> Result<impl Stream<Item = Event<O>>, Error> {
        let mut client = self.etcd.lock().await;
        let (_, stream) = client
            .watch(O::OBJECT_TYPE, Some(WatchOptions::default().with_prefix()))
            .await?;
        Ok(stream.flat_map(|o| {
            futures::stream::iter(if let Ok(o) = o {
                o.events()
                    .iter()
                    .filter_map(|e| {
                        let kv = e.kv()?;
                        Some(match e.event_type() {
                            etcd_client::EventType::Put => {
                                let new = O::parse(kv).ok()?;
                                if let Some(prev) = e.prev_kv() {
                                    let old = O::parse(prev).ok()?;
                                    Event::Update { new, old }
                                } else {
                                    Event::New(new)
                                }
                            }
                            etcd_client::EventType::Delete => {
                                let key = e.kv()?.key();
                                let key = std::str::from_utf8(key).ok()?;
                                let key = if key.len() > O::OBJECT_TYPE.len() + 1 {
                                    key[(O::OBJECT_TYPE.len() + 1)..].to_string()
                                } else {
                                    return None;
                                };
                                Event::Delete(key)
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            } else {
                vec![]
            })
        }))
    }
}

#[derive(Clone, Debug)]
pub enum Event<O> {
    New(O),
    Delete(String),
    Update { new: O, old: O },
}
