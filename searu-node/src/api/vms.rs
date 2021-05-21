use crate::{
    storage::Storage,
    types::{Error, JwtClaim, ListResponse, Project, Vm, VmSpec},
};
use rocket::*;
use rocket_contrib::json::Json;

#[post("/vms", data = "<vm>", format = "json")]
pub async fn create(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
    vm: Json<Vm>,
) -> Result<Json<Vm>, Error> {
    let vm = vm.into_inner();
    storage.store(&vm).await?;
    Ok(vm.into())
}

#[get("/vms")]
pub async fn list(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
) -> Result<Json<ListResponse<Vm>>, Error> {
    let objects = storage.list().await?;
    Ok(ListResponse {
        objects,
        next_page: "".to_string(),
    }
    .into())
}

#[delete("/vms/<name>")]
pub async fn delete(
    storage: State<'_, Storage>,
    name: &str,
    _claim: JwtClaim,
) -> Result<(), Error> {
    storage.delete::<Vm>(name).await?;
    Ok(())
}

pub fn routes() -> Vec<Route> {
    routes![list, create, delete]
}
