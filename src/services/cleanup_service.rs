use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, DatabaseConnection};
use chrono::Utc;
use crate::models::verify_model;

pub async fn clean_expired_tokens(db: &DatabaseConnection) {
    let now = Utc::now().naive_utc();

    match verify_model::Entity::delete_many()
        .filter(verify_model::Column::ExpiresAt.lt(now))
        .exec(db)
        .await
    {
        Ok(res) => {
            if res.rows_affected > 0 {
                println!("Видалено {} прострочених токенів", res.rows_affected);
            }
        }
        Err(e) => eprintln!("Помилка очищення токенів: {e:?}"),
    }
}