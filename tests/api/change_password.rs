use uuid::Uuid;

use crate::helper::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn must_login_to_see_change_password_form() {
    let app = spawn_app().await;

    let resp = app.get_change_password().await;

    assert_is_redirect_to(&resp, "/login");
}

#[tokio::test]
async fn must_login_to_change_password() {
    let app = spawn_app().await;

    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": "something",
            "new_password": "anything-but-a-long-password",
            "new_password_confirmed": "anything-but-a-long-password",
        }))
        .await;

    assert_is_redirect_to(&resp, "/login");
}

#[tokio::test]
async fn new_password_field_must_match() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_confirmed": &another_new_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ))
}

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_confirmed": &new_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password
    });
    let response = app.post_login(&login_body).await;
    assert_is_redirect_to(&response, "/admin/dashboard");
    // Act - Part 2 - Change password
    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_confirmed": &new_password,
        }))
        .await;
    assert_is_redirect_to(&response, "/admin/password");
    // Act - Part 3 - Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));
    // Act - Part 4 - Logout
    let response = app.post_logout().await;
    assert_is_redirect_to(&response, "/login");
    // Act - Part 5 - Follow the redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));
    // Act - Part 6 - Login using the new password
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");
}
