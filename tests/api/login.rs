use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "invalid-username",
        "password": "invalid-password"
    });
    let resp = app.post_login(&login_body).await;

    assert_is_redirect_to(&resp, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));

    let html_page = app.get_login_html().await;
    assert!(!html_page.contains("<p><i>Authentication failed</i></p>"));
}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");

    let html_page = app.get_admin_dashboard().await;
    assert!(html_page.contains(&format!("Welcome {}", &app.test_user.username)));
}
