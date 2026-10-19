//! Access tokens: signed JWTs (HS256) whose subject is an account id.
//!
//! Loco's own `JWT` helper reads the system time, so tokens are encoded here with the time the
//! caller passes in (the injected `Clock`).

use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors;
use crate::shared_lib::error::ApiError;

/// What an access token says. Nothing else goes in a token: no email, password or hash.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Claims {
    /// The account id.
    pub sub: Uuid,
    /// When the token was issued, in seconds since the epoch.
    pub iat: i64,
    /// When the token stops being valid, in seconds since the epoch.
    pub exp: i64,
}

/// Issues a token for `subject`, valid from `now` for `lifetime_secs` seconds, signed with the
/// base64 `secret`.
///
/// # Errors
/// Returns an error if `secret` is not valid base64.
pub(crate) fn issue(
    subject: Uuid,
    now: DateTime<Utc>,
    lifetime_secs: u64,
    secret: &str,
) -> jsonwebtoken::errors::Result<String> {
    let issued_at = now.timestamp();
    let claims = Claims {
        sub: subject,
        iat: issued_at,
        exp: issued_at.saturating_add_unsigned(lifetime_secs),
    };
    jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_base64_secret(secret)?,
    )
}

/// Checks `token` against the base64 `secret` and the time `now` (from the injected `Clock`),
/// and returns its claims.
///
/// # Errors
/// `invalid_token` if the token is malformed or its signature does not match; `token_expired`
/// if its expiry is at or before `now`; `internal_error` if `secret` is not valid base64.
pub(crate) fn verify(token: &str, secret: &str, now: DateTime<Utc>) -> Result<Claims, ApiError> {
    let key = DecodingKey::from_base64_secret(secret).map_err(errors::internal)?;
    // The library would compare `exp` with the system time (and allow leeway). Expiry is
    // checked below against `now`, so the tests and the app share the injected `Clock`.
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    let claims = jsonwebtoken::decode::<Claims>(token, &key, &validation)
        .map_err(|_| errors::invalid_token())?
        .claims;
    if claims.exp <= now.timestamp() {
        return Err(errors::token_expired());
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use uuid::Uuid;

    use super::{issue, verify};

    /// The development secret, base64 as the configuration holds it.
    const SECRET: &str =
        "ZGV2LWFuZC10ZXN0LW9ubHktand0LXNlY3JldC1uZXZlci11c2VkLWluLXByb2R1Y3Rpb24hIQ==";
    const OTHER_SECRET: &str = "b3RoZXItc2VjcmV0LW90aGVyLXNlY3JldC1vdGhlci1zZWNyZXQ=";

    fn start() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 20, 9, 0, 0).unwrap()
    }

    fn code(result: Result<super::Claims, super::ApiError>) -> String {
        result.expect_err("verification fails").body().code.clone()
    }

    #[test]
    fn carries_the_subject_the_issue_time_and_the_expiry() {
        let subject = Uuid::new_v4();
        let token = issue(subject, start(), 86_400, SECRET).unwrap();

        let claims = verify(&token, SECRET, start()).unwrap();
        assert_eq!(claims.sub, subject);
        assert_eq!(claims.iat, start().timestamp());
        assert_eq!(claims.exp, start().timestamp() + 86_400);
    }

    #[test]
    fn carries_nothing_but_the_three_claims() {
        let token = issue(Uuid::new_v4(), start(), 86_400, SECRET).unwrap();

        let payload = jsonwebtoken::dangerous::insecure_decode::<serde_json::Value>(&token)
            .unwrap()
            .claims;
        let mut names: Vec<_> = payload.as_object().unwrap().keys().cloned().collect();
        names.sort();
        assert_eq!(names, ["exp", "iat", "sub"]);
    }

    #[test]
    fn refuses_a_secret_that_is_not_base64() {
        assert!(issue(Uuid::new_v4(), start(), 86_400, "not base64!").is_err());
        let token = issue(Uuid::new_v4(), start(), 86_400, SECRET).unwrap();
        assert_eq!(
            code(verify(&token, "not base64!", start())),
            "internal_error"
        );
    }

    #[test]
    fn a_token_from_another_secret_is_invalid() {
        let token = issue(Uuid::new_v4(), start(), 86_400, SECRET).unwrap();
        assert_eq!(code(verify(&token, OTHER_SECRET, start())), "invalid_token");
    }

    #[test]
    fn a_malformed_token_is_invalid() {
        assert_eq!(code(verify("not-a-jwt", SECRET, start())), "invalid_token");
    }

    #[test]
    fn a_token_with_a_changed_signature_is_invalid() {
        let token = issue(Uuid::new_v4(), start(), 86_400, SECRET).unwrap();
        let (head, signature) = token.rsplit_once('.').unwrap();
        let first = if signature.starts_with('A') { 'B' } else { 'A' };
        let altered = format!("{head}.{first}{}", &signature[1..]);
        assert_eq!(code(verify(&altered, SECRET, start())), "invalid_token");
    }

    #[test]
    fn is_valid_one_second_before_expiry() {
        let token = issue(Uuid::new_v4(), start(), 100, SECRET).unwrap();
        assert!(verify(&token, SECRET, start() + Duration::seconds(99)).is_ok());
    }

    #[test]
    fn is_expired_exactly_at_expiry_and_after() {
        let token = issue(Uuid::new_v4(), start(), 100, SECRET).unwrap();
        assert_eq!(
            code(verify(&token, SECRET, start() + Duration::seconds(100))),
            "token_expired"
        );
        assert_eq!(
            code(verify(&token, SECRET, start() + Duration::days(2))),
            "token_expired"
        );
    }
}
