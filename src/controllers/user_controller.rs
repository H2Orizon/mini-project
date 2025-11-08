use rocket::{State, form::Form, http::{Cookie, CookieJar, SameSite}, request::FlashMessage, response::{Flash, Redirect}};
use rocket_dyn_templates::{Template, context};
use rocket_oauth2::{ OAuth2, TokenResponse};
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::{models::{user_model::{LogIn, ResetsPassword, UserForm}, verify_model::{EmailVerify, PasswordVerify}}, services::{log_service::{get_all_login_logs, log_login_attempt, too_many_failed_attempts}, session::_UserSession, user_service::{activate_email, add_user, chang_twofa, email_2fa, find_or_create_github_user, get_all_user, get_user_by_email, have_token, log_in_user, resets_user_password, send_token}}};

#[derive(rocket::serde::Deserialize)]
pub struct Github;

#[get("/log_in")]
pub fn log_in(flash: Option<FlashMessage<'_>>) -> Template{
    let (flash_msg, flash_kind) = if let Some(f) = &flash{
        (f.message(), f.kind())
    }else {
        ("", "")
    };

    Template::render("user/log_in", context!{
        title: "log in",
        flash_msg,
        flash_kind
    })
}

#[post("/log_in", data= "<form_data>")]
pub async fn post_log_in(db: &State<DatabaseConnection>, form_data: Form<LogIn>, cookies: &CookieJar<'_>) -> Flash<Redirect>{
        let email = form_data.email.clone();

    if too_many_failed_attempts(db, &email).await.unwrap_or(false) {
        return Flash::error(Redirect::to("/log_in"), "Забагато невдалих спроб. Спробуйте пізніше.");
    }

    match log_in_user(&db, form_data.into_inner()).await {
        Ok(user) => {
            if user.twofa_enabled {
                let email = user.email.clone().unwrap();
                cookies.add_private(Cookie::new("email_2fa", email));
                return Flash::warning(Redirect::to("/2fa_verify"), "Введіть код двофакторної автентифікації.");
            }

            cookies.add_private(Cookie::new("user_id", user.id.to_string()));
            let _ = log_login_attempt(db, &email, Some("127.0.0.1".into()), true, Some("Local".into())).await;
            Flash::success(Redirect::to("/admine_panel"), "Ви успішно увійшли!")
        },
        Err(_) => {
            let _ = log_login_attempt(db, &email, Some("127.0.0.1".into()), false, Some("Local".into())).await;
            Flash::error(Redirect::to("/log_in"), "Невірна пошта або пароль.")
        },
    }
}

#[get("/2fa_verify")]
pub async fn email_2fa_verify(flash: Option<FlashMessage<'_>>, db: &State<DatabaseConnection>, cookies: &CookieJar<'_>) -> Template{
    let (flash_msg, flash_kind) = if let Some(f) = &flash{
        (f.message(), f.kind())
    }else {
        ("", "")
    };
    let email = cookies.get_private("email_2fa").map(|c| c.value().to_string()).unwrap_or_default();
    
    if let Some(user_dto) = get_user_by_email(db, email.clone()).await.unwrap_or_default(){
        if user_dto.twofa_enabled && !have_token(db, email.clone()).await{
            send_token(db, email, "Activate Email").await.unwrap()
        }
    }
    Template::render("user/email_verify", context!{
        title: "2fa verify",
        action: uri!("2fa_verify"),
        flash_msg,
        flash_kind
    })
}
#[post("/2fa_verify", data="<form_data>")]
pub async fn post_email_2fa_verify(db: &State<DatabaseConnection>, cookies: &CookieJar<'_>, form_data: Form<EmailVerify>) -> Flash<Redirect>{
    let email = cookies.get_private("email_2fa").map(|c| c.value().to_string()).unwrap_or_default();
    let user = match get_user_by_email(db, email.clone()).await.unwrap() {
        Some(u) => u,
        None => return Flash::error(Redirect::to("/log_in"), "Користувача не знайдено."),
    };
    match email_2fa(db, form_data.token.to_string(), user.id).await {
        Ok(_) => {
            cookies.add_private(Cookie::new("user_id", user.id.to_string()));
            if let Err(e) = log_login_attempt(db, &email, Some("127.0.0.1".to_string()), true, Some("Local".to_string())).await {
                eprintln!("Помилка логування входу: {e}");
            };
            Flash::success(Redirect::to("/admine_panel"), "2FA успішно пройдено.")
        },
        Err(e) => {
            eprintln!("{e}");
            if let Err(e) =  log_login_attempt(db, &email, Some("127.0.0.1".to_string()), false, Some("Local".to_string())).await{
                eprintln!("Помилка логування входу: {e}");
            };
            Flash::error(Redirect::to("/2fa_verify"), "Невірний або прострочений код.")
        }
    }
}

#[get("/register")]
pub fn register(flash: Option<FlashMessage<'_>>) -> Template{
    let (flash_msg, flash_kind) = if let Some(f) = &flash{
        (f.message(), f.kind())
    }else {
        ("", "")
    };
    Template::render("user/register", context!{
        title: "register",
        flash_msg,
        flash_kind
    })
}

#[post("/register", data= "<form_data>")]
pub async fn post_register(db: &State<DatabaseConnection>, form_data: Form<UserForm>) -> Flash<Redirect> {
    match add_user(db, &form_data).await {
        Ok(_) => Flash::success(Redirect::to("/log_in"), "Реєстрація успішна! Увійдіть у свій акаунт."),
        Err(e) => {
            eprintln!("{e}");
            let msg = format!("Не вдалося створити користувача: {e}");
            Flash::error(Redirect::to("/register"), msg)
        }
    }
}

#[post("/toggle_2fa")]
pub async fn toggle_2fa(db: &State<DatabaseConnection>, user_session: Option<_UserSession>) -> Redirect {
    let user_session = match user_session {
        Some(session) => session,
        None => return Redirect::to("/log_in"),
    };
    
    if user_session.user.id == 0 {
        return Redirect::to("/log_in");
    }

    match chang_twofa(user_session.user.id, db).await {
        Ok(_) => Redirect::to("/admine_panel"),
        Err(e) => {
            eprintln!("{e}");
            Redirect::to("/admine_panel")
        }
    }

    
}

#[get("/admine_panel")]
pub async fn admine_panel(user_session: Option<_UserSession>, flash: Option<FlashMessage<'_>>) -> Flash<Result<Template, Redirect>> {
    
    let user_session = match user_session {
        Some(session) => session,
        None => return Flash::error(Err(Redirect::to("/log_in")), "Увійдіть в аккаунт"),
    };

    if user_session.user.id == 0 {
        return Flash::error(Err(Redirect::to("/log_in")), "Увійдіть в аккаунт");
    }
    let (flash_msg, flash_kind) = if let Some(f) = &flash{
        (f.message(), f.kind())
    }else {
        ("", "")
    };
    Flash::success(
            Ok(Template::render("admin/admine_panel", context! {
            title: "Admin Panel",
            user_dto: user_session.user,
            flash_msg,
            flash_kind
        })), "Welcom"
    )
}

#[get("/admine_panel/all_user")]
pub async fn all_user(db: &State<DatabaseConnection>) -> Template {
    let users = get_all_user(db).await.unwrap_or_default();

    Template::render("admin/all_user", context! {
        title: "Login Attempts",
        users: users
    })
}

#[post("/log_out")]
pub fn log_out(cookies: &CookieJar<'_>) -> Redirect{
    cookies.remove_private("user_id");
    Redirect::to(uri!("/"))
}

#[get("/user_verify")]
pub fn user_verify() -> Template {
    Template::render("user/user_verify", context!{
        title: "Password resets",
    })
}

#[get("/email_verify")]
pub async fn email_verify(db: &State<DatabaseConnection>, user_session: Option<_UserSession>) -> Result<Template, Flash<Redirect>>{
    let user_session = match user_session {
        Some(session) => session,
        None => return Err(Flash::error(Redirect::to("/log_in"), "Увійдіть в аккаунт")),
    };
    if user_session.user.is_active {
        return Err(Flash::error(Redirect::to("/admine_panel"), "Аккаунт уже активовано"));
    }

    if !user_session.user.is_active && !have_token(db, user_session.user.email.clone()).await{
        send_token(db, user_session.user.email , "Activate Email").await.unwrap()
    }

    Ok(Template::render("user/email_verify", context!{
        title: "Email verify",
        action: uri!(email_verify)
    }))
}

#[post("/email_verify", data="<form_data>")]
pub async fn post_email_verify(db: &State<DatabaseConnection>, cookies: &CookieJar<'_>, form_data: Form<EmailVerify>) -> Flash<Redirect>{
    let user_id = cookies.get_private("user_id").map(|c| c.value().parse::<i32>().unwrap_or_default()).unwrap_or_default();
    match activate_email(db, form_data.token.clone(), user_id).await {
        Ok(_) => Flash::success(Redirect::to("/admine_panel"), "Електронну пошту успішно підтверджено."),
        Err(_) => Flash::error(Redirect::to("/email_verify"), "Невірний або прострочений токен."),
    }
}

#[post("/rests_token", data = "<form_data>")]
pub async fn creat_resets_token(db: &State<DatabaseConnection>, form_data: Form<PasswordVerify>, cookies: &CookieJar<'_>) -> Redirect{
    let email = form_data.email.clone();

    match send_token(db, email.clone(), "Reset password").await{
        Ok(()) => {
            cookies.add_private(rocket::http::Cookie::new("reset_email", email));
            Redirect::to("/password_resets")
        },
        Err(e) => {
            eprintln!("{e}");
            Redirect::to("/user_verify")
        }
    }
}

#[get("/password_resets")]
pub fn password_resets(flash: Option<FlashMessage<'_>>) -> Template {
    let (flash_msg, flash_kind) = if let Some(f) = &flash{
        (f.message(), f.kind())
    }else {
        ("", "")
    };
    Template::render("user/password_resets", context!{
        title: "Password resets",
        flash_msg,
        flash_kind
    })
}

#[post("/password_resets", data="<form_data>")]
pub async fn post_password_resets(db: &State<DatabaseConnection>, form_data: Form<ResetsPassword>, cookies: &CookieJar<'_>) -> Flash<Redirect> {
    let email = cookies.get_private("reset_email").map(|c| c.value().to_string()).unwrap_or_default();
    if email.is_empty(){
        return Flash::error(Redirect::to("/user_verify"), "Немає активного запиту на скидання паролю.");
    }

    match resets_user_password(db, &form_data, email).await {
        Ok(_) => {
            cookies.remove_private("reset_email");
            Flash::success(Redirect::to("/log_in"), "Пароль успішно змінено.")
        },
        Err(e) => {
            eprintln!("{e}");
            Flash::error(Redirect::to("/password_resets"), "Не вдалося скинути пароль.")
        },
    }
}

#[get("/admine_panel/login_attempts")]
pub async fn view_login_attempts(db: &State<DatabaseConnection>) -> Template {
    let logs = get_all_login_logs(db).await.unwrap_or_default();

    Template::render("admin/login_logs", context! {
        title: "Login Attempts",
        logs: logs
    })
}

#[get("/auth/github")]
pub fn githun_login(oauth2:OAuth2<Github>, cookies: &CookieJar<'_>) -> Redirect{
    oauth2.get_redirect(cookies, &["user:email"]).unwrap()
}

#[get("/auth/github/callback")]
pub async fn github_callback(db: &State<DatabaseConnection>,token: TokenResponse<Github>,cookies: &CookieJar<'_>,) -> Redirect {
    let client = reqwest::Client::new();

    let user_resp = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("token {}", token.access_token()))
        .header("User-Agent", "rocket-app")
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    let github_id = user_resp["id"].as_i64().unwrap().to_string();

    let mut email = user_resp["email"].as_str().map(|s| s.to_string());
    if email.is_none() {
        let emails_resp = client
            .get("https://api.github.com/user/emails")
            .header("Authorization", format!("token {}", token.access_token()))
            .header("User-Agent", "rocket-app")
            .send()
            .await
            .unwrap()
            .json::<Vec<Value>>()
            .await
            .unwrap();

        if let Some(primary) = emails_resp.iter().find(|e| e["primary"].as_bool().unwrap_or(false)){
            email = primary["email"].as_str().map(|s| s.to_string());
        }
    }

    let email = email.unwrap_or_else(|| format!("{}@github.local", github_id));

    let user = find_or_create_github_user(db, github_id, Some(email.clone())).await;
    
    let mut cookie = Cookie::new("user_id", user.id.to_string());
    cookie.set_http_only(true);
    cookie.set_same_site(SameSite::Lax);
    cookies.add_private(cookie);
    if let Err(e) = log_login_attempt(db,&email,Some("127.0.0.1".to_string()),true,Some("GitHub".to_string()),).await{
        eprintln!("Помилка логування входу: {e}");
    };

    Redirect::to(uri!(admine_panel))
}
 
