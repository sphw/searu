use crate::{
    storage::Storage,
    types::{Error, JwtClaim, ListResponse, Project},
};
use rocket::*;
use rocket_contrib::json::Json;

#[post("/projects", data = "<project>", format = "json")]
pub async fn create(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
    project: Json<Project>,
) -> Result<Json<Project>, Error> {
    let project = project.into_inner();
    storage.store(&project).await?;
    Ok(project.into())
}

#[get("/projects")]
pub async fn list(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
) -> Result<Json<ListResponse<Project>>, Error> {
    let objects = storage.list().await?;
    Ok(ListResponse {
        objects,
        next_page: "".to_string(),
    }
    .into())
}

pub fn routes() -> Vec<Route> {
    routes![create, list]
}
