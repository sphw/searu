use crate::{
    auth::Auth,
    storage::Storage,
    types::{Error, JwtClaim, JwtResponse, User, UserSpec},
};
use rocket::*;
use rocket_contrib::json::Json;

#[post("/users", data = "<user>", format = "json")]
pub async fn create(
    storage: State<'_, Storage>,
    _claim: JwtClaim,
    user: Json<UserSpec>,
) -> Result<Json<User>, Error> {
    let user_spec = user.into_inner();
    let user = user_spec.encrypt()?;
    storage.store(&user).await?;
    Ok(user.into())
}

#[post("/users/login", data = "<user>", format = "json")]
pub async fn login(
    storage: State<'_, Storage>,
    auth: State<'_, Auth>,
    user: Json<UserSpec>,
) -> Result<Json<JwtResponse>, Error> {
    let user_spec = user.into_inner();
    let user: User = storage
        .get(&user_spec.username)
        .await?
        .ok_or(Error::Unauthorized)?;
    if !bcrypt::verify(user_spec.password, &user.encrypted_password)
        .map_err(|_| Error::Unauthorized)?
    {
        return Err(Error::Unauthorized);
    }
    let token = auth.create_jwt(user_spec.username)?;
    Ok(JwtResponse { token }.into())
}

pub fn routes() -> Vec<Route> {
    routes![create, login]
}
