use std::env;
use dotenvy::dotenv;
use rocket::{figment::Figment, Config};
use sea_orm::{Database, DatabaseConnection};


pub async fn init_db() -> DatabaseConnection{
    dotenv().ok();
    let url  = env::var("DATABASE_URL").expect("DATABASE_URL isn't established");
    Database::connect(url ).await.expect("Bad connect to db")
}

pub async fn get_figment() -> Figment{
    let secret_key = env::var("SECRET_KEY")
    .unwrap_or_else(|_| "default_secret_key".to_string());
    Config::figment().merge(("secret_key", secret_key))
}
