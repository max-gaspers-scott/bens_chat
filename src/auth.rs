use axum::{
    Json,
    extract::Request,
    http::{StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub exp: usize,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
}

fn jwt_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| "dev-jwt-secret".to_string())
}

pub fn create_token(user_id: Uuid, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp_hours = env::var("JWT_EXP_HOURS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(24);
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        exp: (Utc::now() + Duration::hours(exp_hours)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
}

pub fn validate_token(token: &str) -> Result<AuthUser, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )?;

    let user_id = Uuid::parse_str(&token_data.claims.sub).map_err(|_| {
        jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)
    })?;

    Ok(AuthUser {
        user_id,
        username: token_data.claims.username,
    })
}

pub async fn authorize(
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let Some(header_value) = request.headers().get(AUTHORIZATION) else {
        return Err(unauthorized_response("Missing Authorization header"));
    };

    let Ok(header_value) = header_value.to_str() else {
        return Err(unauthorized_response("Invalid Authorization header"));
    };

    let Some(token) = header_value.strip_prefix("Bearer ") else {
        return Err(unauthorized_response("Invalid bearer token"));
    };

    let auth_user = match validate_token(token) {
        Ok(auth_user) => auth_user,
        Err(_) => return Err(unauthorized_response("Invalid bearer token")),
    };

    request.extensions_mut().insert(auth_user);

    Ok(next.run(request).await)
}

fn unauthorized_response(message: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"status": "error", "error": message})),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_round_trip_preserves_user_identity() {
        let user_id = Uuid::new_v4();
        let token = create_token(user_id, "alice").expect("token should be created");
        let auth_user = validate_token(&token).expect("token should validate");

        assert_eq!(auth_user.user_id, user_id);
        assert_eq!(auth_user.username, "alice");
    }

    #[test]
    fn invalid_token_is_rejected() {
        assert!(validate_token("not-a-real-token").is_err());
    }
}
