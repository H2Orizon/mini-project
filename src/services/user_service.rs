use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::{SaltString, rand_core::OsRng}};
use chrono::{Duration, Utc};
use lettre::{Message, SmtpTransport, Transport, message::Mailbox, transport::smtp::authentication::Credentials};
use rand::{Rng, rng};
use reqwest::Client;
use rocket::figment::Figment;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;
use crate::models::{
    user_model::{ActiveModel as ActiveUser, Entity as User, LogIn, Model as UserModel, ResetsPassword, UserDTO, UserForm},
    verify_model::{self, ActiveModel as ActiveVerify, Entity as Verify}
    };

#[derive(Debug, Error)]
pub enum _UserError {
    #[error("Failed to insert user into database")]
    DatabaseError(#[from] sea_orm::DbErr),
    #[error("PasswordsDoNotMatch")]
    PasswordsDoNotMatch,
    #[error("Validation failed: {0}")]
    ValidationError(validator::ValidationErrors),
    #[error("UserLogInError")]
    UserLogInError,
    #[error("User not found")]
    UserNotFound,
    #[error("CaptchaError")]
    CaptchaError,
}
#[derive(Deserialize)]
struct _RecaptchaResponse {
    success: bool
}

fn _password_hasher(password:&[u8]) -> String{
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    match argon.hash_password(password, &salt) {
        Ok(hash) => hash.to_string(),
        Err(_) => panic!("Password hashing failed"),
    }
}

fn _verify_password(password: &str, hash_pass: &str) -> bool{
    let argon = Argon2::default();
    if let Ok(parsed_hash) = PasswordHash::new(hash_pass){
        argon.verify_password(password.as_bytes(), &parsed_hash).is_ok()
    }else {
        false
    }
}

fn _generate_token() -> String{
    let abc: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890".chars().collect();
    (0..10).map(|_| abc[rng().random_range(0..abc.len())]).collect()
}

async fn _verify_captcha(captcha_token: &str, captcha_secret: &str) -> bool{
    let cln = Client::new();
    let res = cln.post("https://www.google.com/recaptcha/api/siteverify")
    .form(&[("secret",captcha_secret), ("response",captcha_token)])
    .send().await.unwrap();
    let json: _RecaptchaResponse = res.json().await.unwrap();
    json.success
}

async fn delete_all_user_token_by_user_id(db: &DatabaseConnection, user_id: i32) -> Result<(), DbErr> {
    Verify::delete_many().filter(verify_model::Column::UserId.eq(user_id)).exec(db).await?;
    Ok(())
}

async fn get_user_token_by_email(db: &DatabaseConnection, email:String) -> Result<String, _UserError>{
    let user = get_user_by_email(db, email).await
    .map_err(|e| e)?
    .ok_or(_UserError::UserNotFound)?;
    let token = Verify::find().filter(verify_model::Column::UserId.eq(user.id)).one(db).await
    .map_err(|e| _UserError::DatabaseError(e))?
    .ok_or(_UserError::UserNotFound)?;
    Ok(token.token_hash)
}

pub async fn have_token(db: &DatabaseConnection, email:String) -> bool{
    match get_user_token_by_email(db, email).await {
        Ok(_) => return true,
        Err(_) => return false
    }
}

pub async fn add_user(db: &DatabaseConnection, form_data: &UserForm) -> Result<(), _UserError>{
    form_data.validate().map_err(|e| _UserError::ValidationError(e))?;

    if form_data.password != form_data.password2 {
        return Err(_UserError::PasswordsDoNotMatch);
    }

    if !_verify_captcha(&form_data.g_recaptcha_response, "6LdqaQAsAAAAAFBGR4GflpejSXi2TYXlaieVM6-f").await{
        return Err(_UserError::CaptchaError);
    }
    
    ActiveUser{
        email: Set(Some(form_data.email.clone())),
        password: Set(Some(_password_hasher(form_data.password.as_bytes()))),
        is_active: Set(false),
        ..Default::default()
    }.insert(db).await.map_err(_UserError::DatabaseError)?;

    send_token(db, form_data.email.clone(), "Activate Email").await.unwrap();

    Ok(())
}

pub async fn log_in_user(db: &DatabaseConnection, form_data:LogIn) -> Result<UserModel, _UserError> {
    if let Some(user) = get_user_by_email(db, form_data.email).await?{
        if _verify_password(&form_data.password.clone(), &user.password.clone().unwrap()){
            Ok(user)
        }else {
            Err(_UserError::UserLogInError)
        }
    }else {
        Err(_UserError::UserLogInError)
    }
}

pub async fn get_user_by_id(db: &DatabaseConnection, user_id: i32) -> Result<UserModel, _UserError> {
    let user = User::find_by_id(user_id).one(db).await        
    .map_err(|e| _UserError::DatabaseError(e))?
    .ok_or(_UserError::UserNotFound)?;
    Ok(user)
}

pub async fn get_user_by_email(db: &DatabaseConnection, email:String) -> Result<Option<UserModel>, _UserError> {
    let user = User::find()
    .filter(<User as EntityTrait>::Column::Email.eq(email))
    .one(db).await.map_err(|e| _UserError::DatabaseError(e))?;
    Ok(user)
}

pub async fn get_all_user(db: &DatabaseConnection,) -> Result<Vec<UserDTO>, sea_orm::DbErr>{
    let users = User::find().all(db).await?;

    let user_dtos = users.into_iter()
    .map(|user| UserDTO{
        id: user.id,
        email: user.email.clone().unwrap(),
        is_active: user.is_active,
        oauth_provider: user.oauth_provider,
        twofa_enabled: user.twofa_enabled
    }).collect();

    Ok(user_dtos)
}

pub async fn send_mail(figment: &Figment, to: &str,subject: &str,body: &str) -> Result<(), String> {
    let smtp_host: String = figment.extract_inner("smtp_host").unwrap();
    let smtp_username: String = figment.extract_inner("smtp_username").unwrap();
    let smtp_password: String = figment.extract_inner("smtp_password").unwrap();
    let from_email: String = figment.extract_inner("from_email").unwrap();

    let email = Message::builder()
    .from(from_email.parse::<Mailbox>().unwrap())
    .to(to.parse::<Mailbox>().unwrap())
    .subject(subject)
    .body(body.to_string())
    .unwrap();

    let creds = Credentials::new(smtp_username, smtp_password);

    let mailer = SmtpTransport::starttls_relay(&smtp_host)
    .unwrap()
    .credentials(creds)
    .build();

    match mailer.send(&email) {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => eprintln!("Could not send email: {:?}", e),
    }
    Ok(())
}

pub async fn send_token(db: &DatabaseConnection, email:String, subject: &str) -> Result<(), _UserError>{
    if let Some(user) = get_user_by_email(db, email.clone()).await?{

        delete_all_user_token_by_user_id(db, user.id).await?;

        let token = _generate_token();
        match send_mail(&rocket::Config::figment(), &email, subject, &token).await {
            Ok(_) => println!("Email успішно відправлено!"),
            Err(e) => eprintln!("Mail error: {e}"),
        }

        let time_now = Utc::now();
        let expires_at = time_now + Duration::minutes(1);

        ActiveVerify{
            user_id: Set(user.id),
            token_hash: Set(_password_hasher(token.as_bytes())),
            expires_at: Set(expires_at.naive_utc()),
            ..Default::default()
        }.insert(db).await.map_err(_UserError::DatabaseError)?;
        Ok(())
    }else {
        Err(_UserError::UserLogInError)
    }
}

pub async fn resets_user_password(db: &DatabaseConnection, form_data: &ResetsPassword, email:String) -> Result<(), _UserError>{
    form_data.validate().map_err(|e| _UserError::ValidationError(e))?;

    if form_data.password != form_data.password2{
        return Err(_UserError::PasswordsDoNotMatch);
    }

    if !_verify_captcha(&form_data.g_recaptcha_response, "6LdqaQAsAAAAAFBGR4GflpejSXi2TYXlaieVM6-f").await {
        return Err(_UserError::CaptchaError);
    }

    let token_hesh = get_user_token_by_email(db, email.clone()).await.unwrap_or_default();

    if !_verify_password(&form_data.token, &token_hesh){
        return Err(_UserError::CaptchaError);
    };

    let new_password = _password_hasher(form_data.password.as_bytes());

    let user = get_user_by_email(db, email).await?.unwrap();
    delete_all_user_token_by_user_id(db, user.id).await?;

    let mut user_edit: ActiveUser = user.into();
    user_edit.password = Set(Some(new_password));
    user_edit.update(db).await.map_err(_UserError::DatabaseError)?;

    Ok(())
}

async fn is_verify(token: String, db: &DatabaseConnection, user:UserModel) -> bool {
    let token_hesh = get_user_token_by_email(db, user.email.unwrap()).await.unwrap_or_default();
    if !_verify_password(&token, &token_hesh){
        return false;
    }
    let _ = delete_all_user_token_by_user_id(db, user.id).await;
    true
}

pub async fn email_2fa(db: &DatabaseConnection, token: String, user_id:i32) -> Result<(), _UserError> {
    let user = get_user_by_id(db, user_id).await?;
    if !is_verify(token,db,user.clone()).await{
        return Err(_UserError::CaptchaError);
    }
    Ok(())
}

pub async fn chang_twofa(user_id: i32, db: &DatabaseConnection) -> Result<(), _UserError>{
    let user =get_user_by_id(db, user_id).await?;
    let new_state = !user.twofa_enabled.clone();

    let mut user_edit: ActiveUser = user.into();
    user_edit.twofa_enabled = Set(new_state);
    user_edit.update(db).await.map_err(_UserError::DatabaseError)?;
    Ok(())
}

pub async fn activate_email (db: &DatabaseConnection, token: String, user_id:i32) -> Result<(), _UserError>{
    let user = get_user_by_id(db, user_id).await?;
    if !is_verify(token,db,user.clone()).await{
        return Err(_UserError::CaptchaError);
    }
    let mut user_edit: ActiveUser = user.into();
    user_edit.is_active = Set(true);
    user_edit.update(db).await.map_err(_UserError::DatabaseError)?;

    Ok(())
}

pub async fn find_or_create_github_user(db: &DatabaseConnection, github_id: String, email:Option<String>) -> UserModel{
    if let Ok(Some(extension)) = User::find().filter(<User as EntityTrait>::Column::OauthId.eq(github_id.clone())).one(db).await{
        return extension;
    }
    let new_user = ActiveUser{
        email: Set(email),
        password: Set(None),
        is_active: Set(true),
        oauth_provider: Set(Some("github".to_string())),
        oauth_id: Set(Some(github_id)),
        ..Default::default()
    };
    new_user.insert(db).await.unwrap()
}
