use axum::response::IntoResponse;
use axum::{Extension, Form};
use axum::http::StatusCode;
use rand::Rng;
use serde::Deserialize;

use crate::auth::{AdminSecret, AdminSessions};

// Login / Logout

#[derive(Deserialize)]
pub struct LoginForm {
    pub secret: String,
}

pub async fn login_submit(
    Extension(admin_secret): Extension<AdminSecret>,
    Extension(sessions): Extension<AdminSessions>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    if admin_secret.0.is_empty() || form.secret != admin_secret.0 {
        return crate::admin::pages::login_page_with_error("Invalid admin secret").into_response();
    }

    let token: String = {
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    };
    let expires_at = chrono::Utc::now().timestamp() + 28800;

    sessions.0.write().await.insert(token.clone(), expires_at);

    let cookie = format!(
        "oj_admin_session={}; Path=/admin; HttpOnly; SameSite=Lax; Max-Age=28800",
        token
    );

    (
        StatusCode::SEE_OTHER,
        [("location", "/admin/"), ("set-cookie", &cookie)],
    )
        .into_response()
}

pub async fn logout(
    Extension(sessions): Extension<AdminSessions>,
    request: axum::extract::Request,
) -> impl IntoResponse {
    if let Some(token) = crate::auth::extract_cookie(request.headers(), "oj_admin_session") {
        sessions.0.write().await.remove(token);
    }

    let cookie = "oj_admin_session=; Path=/admin; HttpOnly; SameSite=Lax; Max-Age=0";

    (
        StatusCode::SEE_OTHER,
        [("location", "/admin/login"), ("set-cookie", cookie)],
    )
        .into_response()
}
