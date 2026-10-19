//! `GET /api/me`.

use axum::Json;
use loco_rs::prelude::*;

use super::dto::ProfileResponse;
use crate::modules::identity::auth::Authenticated;

pub(super) fn routes() -> Routes {
    Routes::new().prefix("api").add("/me", get(me))
}

async fn me(Authenticated(account): Authenticated) -> Json<ProfileResponse> {
    Json(account.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::identity::test_support::{database, insert_account};

    #[tokio::test]
    async fn returns_the_callers_own_profile() {
        let db = database().await;
        let alex = insert_account(&db, "alex@example.edu", "ACTIVE").await;
        let id = alex.id;

        let Json(profile) = me(Authenticated(alex)).await;

        assert_eq!(profile.id, id);
        assert_eq!(profile.email, "alex@example.edu");
        assert_eq!(profile.full_name, "Alex Tan");
        assert_eq!(profile.status, "ACTIVE");
    }
}
