use sea_orm::entity::prelude::*;
use validator::Validate;

use crate::models::user_model;

#[derive(Clone,Debug,DeriveEntityModel)]
#[sea_orm(table_name="password_resets")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id:i32,
    pub user_id:i32,
    pub token_hash:String,
    pub expires_at:DateTime,
    #[sea_orm(default_value = false)]
    pub used:bool
}

#[derive(FromForm, Validate)]
pub struct EmailVerify{
    pub token: String
}

#[derive(FromForm, Validate)]
pub struct PasswordVerify{
    pub email: String
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "user_model::Entity",
        from = "Column::UserId",
        to = "user_model::Column::Id"
    )]
    User,
}

impl Related<user_model::Entity> for Entity{
    fn to() -> RelationDef{
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}