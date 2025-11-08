use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use crate::models::login_attempts;
use login_attempts::{ActiveModel, Entity};

pub async fn log_login_attempt(db: &DatabaseConnection, email: &str, ip: Option<String>, success: bool, provider:Option<String>) -> Result<(), DbErr>{
    let new_log = ActiveModel{
        user_email: Set(email.to_string()),
        ip_address: Set(ip),
        success: Set(success),
        created_at: Set(Utc::now()),
        provider: Set(provider),
        ..Default::default()
    };
    new_log.insert(db).await?;
    Ok(())
}

pub async fn too_many_failed_attempts(db: &DatabaseConnection, email: &str) -> Result<bool, DbErr>{
    let attempt = Entity::find().filter(login_attempts::Column::UserEmail.eq(email))
    .order_by_desc(login_attempts::Column::CreatedAt)
    .limit(5)
    .all(db)
    .await?;

    Ok(attempt.len() == 5 && attempt.iter().all(|a| !a.success))
}

pub async fn get_all_login_logs(db: &DatabaseConnection) -> Result<Vec<login_attempts::Model>, DbErr>{
    let logs = login_attempts::Entity::find().order_by_desc(login_attempts::Column::CreatedAt).all(db).await.unwrap_or_default();
    Ok(logs)
}
