use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};
use serde::Deserialize;
use subtle::ConstantTimeEq;

use crate::error::AppError;

const COOKIE_NAME: &str = "auth";

#[derive(Deserialize)]
pub struct LoginBody {
    pub password: String,
}

pub async fn login(jar: SignedCookieJar, Json(body): Json<LoginBody>) -> Response {
    let expected = std::env::var("APP_PASSWORD").unwrap_or_default();
    let ok: bool = body
        .password
        .as_bytes()
        .ct_eq(expected.as_bytes())
        .into();
    if !expected.is_empty() && ok {
        let cookie = Cookie::build((COOKIE_NAME, "1"))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(true)
            .build();
        (jar.add(cookie), StatusCode::NO_CONTENT).into_response()
    } else {
        AppError::Unauthorized.into_response()
    }
}

pub async fn logout(jar: SignedCookieJar) -> Response {
    // Removal cookie must carry the same path as the one set at login, or the browser won't clear it.
    let jar = jar.remove(Cookie::build((COOKIE_NAME, "")).path("/").build());
    (jar, StatusCode::NO_CONTENT).into_response()
}

pub struct AuthUser;

impl<S> FromRequestParts<S> for AuthUser
where
    Key: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = SignedCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|_| AppError::Unauthorized)?;
        match jar.get(COOKIE_NAME) {
            Some(c) if c.value() == "1" => Ok(AuthUser),
            _ => Err(AppError::Unauthorized),
        }
    }
}
