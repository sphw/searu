mod node_info;
mod scheduler;
mod vm_supervisor;
mod watcher;
pub use node_info::*;
pub use scheduler::*;
pub use vm_supervisor::*;
pub use watcher::*;

use std::time::Duration;

use tokio::{
    sync::{
        mpsc::{self, Sender},
        oneshot,
    },
    task::JoinHandle,
};

use crate::types::Error;

#[async_trait::async_trait]
pub trait Actor {
    type Message;
    type Response;

    async fn handle(&mut self, message: Self::Message) -> Result<Self::Response, Error>;

    async fn init(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn spawn(mut self) -> (Handle<Self>, JoinHandle<Result<(), anyhow::Error>>)
    where
        Self: Send + Sync + Sized + 'static,
        Self::Message: Send + Sync,
        Self::Response: Send + Sync,
    {
        let (tx, mut rx) = mpsc::channel(100);
        let task = tokio::spawn(async move {
            self.init().await?;
            while let Some(pair) = rx.recv().await {
                let (msg, resp_tx): (_, oneshot::Sender<Result<Self::Response, Error>>) = pair;
                let resp = self.handle(msg).await;
                let _ = resp_tx.send(resp);
            }
            Ok(())
        });
        (Handle(tx), task)
    }

    fn repeat(mut self, duration: Duration) -> JoinHandle<Result<(), anyhow::Error>>
    where
        Self: Send + Sync + Sized + 'static,
        Self::Message: Send + Default,
    {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(duration);
            loop {
                let _ = self.handle(Default::default()).await?;
                interval.tick().await;
            }
        })
    }
}

type ActorSender<Message, Response> = Sender<(Message, oneshot::Sender<Result<Response, Error>>)>;
pub struct Handle<A: Actor>(ActorSender<A::Message, A::Response>);

impl<A: Actor> Handle<A> {
    async fn send(&self, msg: A::Message) -> Result<A::Response, Error> {
        let (tx, rx) = oneshot::channel();
        self.0.send((msg, tx)).await.map_err(|_| Error::ActorSend)?;
        let resp = rx.await?;
        resp
    }
}
