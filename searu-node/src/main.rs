use std::time::Duration;

use actors::{Actor, NodeInfo, Scheduler, VmSupervisor, VmWatcher, VpcSupervisor, VpcWatcher};
use types::{Project, UserSpec};

mod actors;
mod api;
mod auth;
mod config;
mod storage;
mod types;
pub mod vmm;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let config = config::Config::new()?;
    let client = etcd_client::Client::connect([&config.etcd_addr], None).await?;
    let storage = storage::Storage::new(client);
    let auth = auth::Auth::new(&config.jwt_secret)?;
    storage
        .store(&UserSpec::new("admin".to_string(), "admin".to_string()).encrypt()?)
        .await?;
    storage
        .store(&Project {
            name: "default".to_string(),
        })
        .await?;
    let node_info = NodeInfo::new(storage.clone()).repeat(Duration::from_secs(60));
    let (scheduler, scheduler_handle) = Scheduler::new(storage.clone()).spawn();
    let (netlink_conn, netlink_handle, _) = rtnetlink::new_connection().unwrap();
    let netlink_conn = tokio::spawn(async {
        netlink_conn.await;
        Ok::<_, anyhow::Error>(())
    });
    let vm_supervisor = VmSupervisor::new(storage.clone(), netlink_handle.clone())?;
    let (vm_supervisor, vm_supervisor_handle) = vm_supervisor.spawn();
    let vm_watcher = VmWatcher::new(storage.clone(), scheduler.clone(), vm_supervisor).spawn();

    let (vpc_supervisor, vpc_supervisor_handle) =
        VpcSupervisor::new(storage.clone(), netlink_handle).spawn();
    let vpc_watcher = VpcWatcher::new(storage.clone(), scheduler, vpc_supervisor).spawn();
    let rocket = tokio::spawn(async {
        rocket::build()
            .manage(storage)
            .manage(config)
            .manage(auth)
            .mount("/api", api::routes())
            .ignite()
            .await?
            .launch()
            .await?;
        Ok::<_, anyhow::Error>(())
    });
    let _ = futures::future::select_all(vec![
        node_info,
        rocket,
        vm_supervisor_handle,
        vm_watcher,
        vpc_supervisor_handle,
        vpc_watcher,
        scheduler_handle,
        netlink_conn,
    ])
    .await
    .0?;
    println!("exiting");
    Ok(())
}
