use crate::types::{Error, InnerJwtClaim, JwtClaim};
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};

pub struct Auth {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey<'static>,
}

impl Auth {
    pub fn new(secret: &str) -> Result<Self, Error> {
        Ok(Self {
            encoding_key: EncodingKey::from_base64_secret(secret)?,
            decoding_key: DecodingKey::from_base64_secret(secret)?.into_static(),
        })
    }

    pub fn create_jwt(&self, username: String) -> Result<String, Error> {
        let header = Header::new(Algorithm::HS512);
        let exp = Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .expect("valid timestamp")
            .timestamp();
        let claim = JwtClaim {
            inner: InnerJwtClaim::User(username),
            exp,
        };
        Ok(encode(&header, &claim, &self.encoding_key)?)
    }

    pub fn parse_jwt(&self, token: &str) -> Result<JwtClaim, Error> {
        println!("parse jwt");
        let data = decode::<JwtClaim>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS512),
        )?;
        Ok(data.claims)
    }
}
