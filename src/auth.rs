use axum::extract::{FromRef, FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_extra::extract::cookie::{Cookie, Key, SameSite, SignedCookieJar};
use serde::Deserialize;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;

use crate::app::AppState;
use crate::error::AppError;

const COOKIE_NAME: &str = "auth";

// Brute-force throttle: after MAX_FAILS consecutive wrong passwords, reject ALL
// login attempts for LOCKOUT. Global (not per-IP) — the app has one shared
// password, and a global counter can't be evaded by spoofing X-Forwarded-For
// behind the proxy. ponytail: global lockout means an active attacker also
// locks out the owner until LOCKOUT expires; acceptable for a single-user tool.
// Upgrade path if that DoS matters: key by trusted proxy-supplied client IP.
const MAX_FAILS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
pub struct LoginThrottle {
    fails: u32,
    locked_until: Option<Instant>,
}

#[derive(Deserialize)]
pub struct LoginBody {
    pub password: String,
}

pub async fn login(
    State(s): State<AppState>,
    jar: SignedCookieJar,
    Json(body): Json<LoginBody>,
) -> Response {
    // Refuse while locked out (no password check happens during lockout).
    {
        let t = s.throttle.lock().unwrap();
        if t.locked_until.is_some_and(|until| Instant::now() < until) {
            return (StatusCode::TOO_MANY_REQUESTS, "too many attempts, try later").into_response();
        }
    }

    let expected = std::env::var("APP_PASSWORD").unwrap_or_default();
    let ok: bool = body
        .password
        .as_bytes()
        .ct_eq(expected.as_bytes())
        .into();
    if !expected.is_empty() && ok {
        let mut t = s.throttle.lock().unwrap();
        t.fails = 0;
        t.locked_until = None;
        drop(t);
        let cookie = Cookie::build((COOKIE_NAME, "1"))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(true)
            .build();
        (jar.add(cookie), StatusCode::NO_CONTENT).into_response()
    } else {
        let mut t = s.throttle.lock().unwrap();
        t.fails += 1;
        if t.fails >= MAX_FAILS {
            t.fails = 0;
            t.locked_until = Some(Instant::now() + LOCKOUT);
        }
        drop(t);
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
