#[macro_use] extern crate rocket;
use std::time::Duration;

use rocket::{fs::FileServer, launch, response::Redirect};
use rocket_dyn_templates::{context, Template};
use rocket_oauth2::OAuth2;
use crate::{controllers::user_controller::{Github, all_user, creat_resets_token, email_2fa_verify, email_verify, github_callback, githun_login, log_out, password_resets, post_email_2fa_verify, post_email_verify, post_password_resets, toggle_2fa, user_verify, view_login_attempts}, db::{get_figment, init_db}, services::cleanup_service::clean_expired_tokens};
use controllers::user_controller::{log_in, admine_panel, register, post_log_in, post_register};

mod db;
mod controllers;
mod services;
mod models;
mod validators;

#[get("/")]
fn index() -> Redirect{
    Redirect::to("/log_in")
}
#[get("/test")]
async fn test_page() -> Template {
    let token = /*creat_token().await*/ "asdf";

    Template::render("test_page", context! {
        title: "test_page",
        message: token
    })
}

#[launch]
async fn rocket() -> _ {
    let db = init_db().await;

    let db_clone = db.clone();
    tokio::spawn(async move {
        loop {
            println!("Running background cleanup...");
            clean_expired_tokens(&db_clone).await;
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });

    rocket::custom(get_figment().await)
    .manage(db)
    .mount("/", routes![index, test_page
        , log_in, post_log_in
        , register, post_register
        , admine_panel, log_out
        , password_resets, post_password_resets
        , user_verify, creat_resets_token
        , email_verify, post_email_verify
        , view_login_attempts, all_user
        , githun_login, github_callback
        , toggle_2fa , email_2fa_verify, post_email_2fa_verify
    ])
    .mount("/static", FileServer::from("static"))
    .attach(OAuth2::<Github>::fairing("github"))
    .attach(Template::fairing())
}