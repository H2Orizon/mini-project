use rocket::{Request, request::{FromRequest, Outcome}};
use rocket::http::Status;
use sea_orm::DatabaseConnection;
use serde::Serialize;

use crate::services::user_service::get_user_by_id;
use crate::models::user_model::UserDTO;

#[derive(Serialize)]
pub struct _UserSession {
    pub user: UserDTO,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for _UserSession {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = req.cookies();
        let db = req.rocket().state::<DatabaseConnection>().unwrap();

        if let Some(cookie) = cookies.get_private("user_id") {
            if let Ok(id) = cookie.value().parse::<i32>() {
                if let Ok(user) = get_user_by_id(db, id).await {
                    let dto = UserDTO {
                        id: user.id,
                        email: user.email.clone().unwrap_or_else(|| "unknown@oauth.user".to_string()),
                        is_active: user.is_active,
                        oauth_provider: user.oauth_provider,
                        twofa_enabled: user.twofa_enabled
                    };
                    return Outcome::Success(_UserSession { user: dto });
                }
            }
        }
        Outcome::Error((Status::Unauthorized, ()))
    }
}