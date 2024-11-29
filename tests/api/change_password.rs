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
            "new_password": "anything",
            "new_password_confirmed": "anything",
        }))
        .await;

    assert_is_redirect_to(&resp, "/login");
}
