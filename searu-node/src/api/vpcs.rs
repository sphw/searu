use crate::{
    storage::Storage,
    types::{Error, JwtClaim, ListResponse, Vpc},
};
use rocket::*;
use rocket_contrib::json::Json;

#[post("/vpcs", data = "<vpc>", format = "json")]
pub async fn create(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
    vpc: Json<Vpc>,
) -> Result<Json<Vpc>, Error> {
    let vpc = vpc.into_inner();
    storage.store(&vpc).await?;
    Ok(vpc.into())
}

#[get("/vpcs")]
pub async fn list(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
) -> Result<Json<ListResponse<Vpc>>, Error> {
    let objects = storage.list().await?;
    Ok(ListResponse {
        objects,
        next_page: "".to_string(),
    }
    .into())
}

#[delete("/vpcs/<name>")]
pub async fn delete(
    storage: State<'_, Storage>,
    name: &str,
    _claim: JwtClaim,
) -> Result<(), Error> {
    storage.delete::<Vpc>(name).await?;
    Ok(())
}

pub fn routes() -> Vec<Route> {
    routes![list, create, delete]
}
