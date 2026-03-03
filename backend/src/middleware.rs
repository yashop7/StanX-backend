use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::env;

use crate::routes::user::Claims;

pub struct JwtClaims(pub Claims); // we can access this by jwt_claims.0

// pub struct JwtClaims { // we will access it by jwt_claims.claims
//     pub claims: Claims
// }

#[async_trait]
impl<S> FromRequestParts<S> for JwtClaims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get("Authorization");

        if let Some(header_value) = auth_header {
            if let Ok(token) = header_value.to_str() {
                let secret = env::var("SECRET_KEY")
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "JWT_SECRET must be set".to_string()))?;
                let decoding_key = DecodingKey::from_secret(secret.as_bytes());
                let validation = Validation::default();

                match decode::<Claims>(token, &decoding_key, &validation) {
                    Ok(token_data) => {
                        return Ok(JwtClaims(token_data.claims));
                    }
                    Err(e) => {
                        eprintln!("JWT decoding error: {:?}", e);
                        return Err((StatusCode::UNAUTHORIZED, "Invalid JWT token".to_string()));
                    }
                }
            }
        }
        Err((StatusCode::UNAUTHORIZED, "Authorization header missing or invalid".to_string()))
    }
}