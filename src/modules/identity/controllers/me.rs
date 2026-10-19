//! `GET /api/me`.

use axum::Json;
use loco_rs::prelude::*;

use super::authenticated::Authenticated;
use crate::modules::identity::dtos::ProfileResponse;

pub(super) fn routes() -> Routes {
    Routes::new().prefix("api").add("/me", get(me))
}

async fn me(Authenticated(account): Authenticated) -> Json<ProfileResponse> {
    Json(account.into())
}
