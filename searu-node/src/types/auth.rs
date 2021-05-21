use rocket::{request::Outcome, State};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use super::{Error, Metadata, Object};

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub encrypted_password: String,
}

impl Object for User {
    const OBJECT_TYPE: &'static str = "user";

    fn metadata(&self) -> Cow<'_, Metadata> {
        Cow::Owned(Metadata {
            name: self.username.clone(),
            project: "".to_string(),
            version: None,
        })
    }

    fn set_version(&mut self, _: i64) {}
}

#[derive(Serialize, Deserialize)]
pub struct UserSpec {
    pub username: String,
    pub password: String,
}

impl UserSpec {
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    pub(crate) fn encrypt(self) -> Result<User, Error> {
        Ok(User {
            username: self.username,
            encrypted_password: bcrypt::hash(self.password, bcrypt::DEFAULT_COST)?,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct JwtClaim {
    pub inner: InnerJwtClaim,
    pub exp: i64,
}

#[derive(Serialize, Deserialize)]
pub enum InnerJwtClaim {
    User(String),
}

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for JwtClaim {
    type Error = Error;

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        if let Some(auth) = request
            .guard::<State<crate::auth::Auth>>()
            .await
            .succeeded()
        {
            if let Some(header) = request.headers().get_one("Authorization") {
                if let Some(token) = header.splitn(2, "Bearer ").nth(1) {
                    if let Ok(claim) = auth.parse_jwt(token) {
                        return Outcome::Success(claim);
                    }
                }
            }
        }
        Outcome::Failure((rocket::http::Status::Unauthorized, Error::Unauthorized))
    }
}

#[derive(Serialize, Deserialize)]
pub struct JwtResponse {
    pub token: String,
}
