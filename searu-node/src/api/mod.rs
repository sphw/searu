use rocket::*;

mod nodes;
mod projects;
mod users;
mod vms;
mod vpcs;

#[get("/")]
pub fn index() -> &'static str {
    "v0.0.1"
}

pub fn routes() -> Vec<Route> {
    let mut routes = routes![index];
    routes.append(&mut users::routes());
    routes.append(&mut projects::routes());
    routes.append(&mut nodes::routes());
    routes.append(&mut vms::routes());
    routes.append(&mut vpcs::routes());
    routes
}
