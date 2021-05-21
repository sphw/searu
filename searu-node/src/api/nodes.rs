use crate::{
    storage::Storage,
    types::{Error, JwtClaim, ListResponse, Node},
};
use rocket::*;
use rocket_contrib::json::Json;

#[get("/nodes")]
pub async fn list(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
) -> Result<Json<ListResponse<Node>>, Error> {
    let objects = storage.list().await?;
    Ok(ListResponse {
        objects,
        next_page: "".to_string(),
    }
    .into())
}

#[get("/nodes/<id>")]
pub async fn get(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
    id: String,
) -> Result<Json<Node>, Error> {
    let node: Node = storage
        .get(&id)
        .await?
        .ok_or(Error::NotFound(format!("node: {}", id)))?;
    Ok(node.into())
}

pub fn routes() -> Vec<Route> {
    routes![list, get]
}
