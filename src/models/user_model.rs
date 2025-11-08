use sea_orm::entity::prelude::*;
use serde::Serialize;
use validator::Validate;
use crate::{validators::password_validator::_validator_password};


#[derive(Clone, Debug, DeriveEntityModel)]
#[sea_orm(table_name="user")]
pub struct Model{
    #[sea_orm(primary_key)]
    pub id:i32,
    pub email:Option<String>,
    pub password:Option<String>,
    #[sea_orm(default_value = false)]
    pub is_active:bool,

    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    
    #[sea_orm(default_value = false)]
    pub twofa_enabled: bool
}

#[derive(FromForm, Validate)]
pub struct UserForm{
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    #[validate(custom(function = "_validator_password"))]
    pub password: String,
    pub password2: String,
    #[field(name = "g-recaptcha-response")]
    pub g_recaptcha_response: String,
}

#[derive(FromForm, Validate)]
pub struct LogIn{
    #[validate(email)]
    pub email: String,
    pub password: String
}

#[derive(FromForm, Validate)]
pub struct ResetsPassword{
    pub token: String,
    #[validate(length(min = 8))]
    #[validate(custom(function = "_validator_password"))]
    pub password: String,
    pub password2: String,
    #[field(name = "g-recaptcha-response")]
    pub g_recaptcha_response: String,
}

#[derive(Serialize)]
pub struct UserDTO{
    pub id:i32,
    pub email: String,
    pub is_active:bool,
    pub oauth_provider: Option<String>,
    pub twofa_enabled: bool
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}