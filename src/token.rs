use jsonwebtoken::{EncodingKey, Header, encode, errors::Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub audio_uri: String,
}

impl Claims {
    pub fn new(audio_uri: String) -> Claims {
        Claims { audio_uri }
    }

    pub fn to_token(&self, key: &str) -> Result<String, Error> {
        encode(&Header::default(), self, &EncodingKey::from_secret(key.as_ref()))
    }
}
