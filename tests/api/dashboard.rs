use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    // Arrange
    let app = spawn_app().await;
    // Act
    let response = app.get_admin_dashboard().await;
    // Assert
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    let resp = app.post_logout().await;
    assert_is_redirect_to(&resp, "/login");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains(&format!("<p><i>You have successfully logged out.</i></p>")));

    let resp = app.get_admin_dashboard().await;
    assert_is_redirect_to(&resp, "/login");
}
