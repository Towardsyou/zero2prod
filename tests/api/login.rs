use crate::helper::spawn_app;

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let login_body = serde_json::json!({
        "username": "invalid-username",
        "password": "invalid-password"
    });
    let resp = app.post_login(&login_body).await;

    assert_eq!(resp.status().as_u16(), 303);
    assert_eq!(
        resp.headers()
            .get(reqwest::header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap(),
        "/login"
    );

    let flash_cookie = resp.cookies().find(|c| c.name() == "_flash").unwrap();
    assert!(flash_cookie.value() == "Authentication failed");

    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>Authentication failed</i></p>"));
}
