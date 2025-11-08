use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name = "login_attempts")]
#[derive(serde::Serialize)]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_email: String,
    pub ip_address: Option<String>,
    pub success: bool,
    pub created_at: DateTimeUtc,
    pub provider: Option<String>
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}