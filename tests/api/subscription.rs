use crate::helper::spawn_app;

#[tokio::test]
async fn subscribe_return_200_for_valid_input() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = app.post_subscriptions(body.to_owned()).await;

    assert!(
        response.status().is_success(),
        "Unsuccessful status code {} message: {:?}",
        response.status().as_u16(),
        response.text().await
    );

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_return_40x_for_invalid_input() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=le%20guin&email=", "empty email"),
        ("name=le%20guin&email=not-an-email", "invalid email"),
    ];

    for (body, desc) in test_cases {
        let response = app.post_subscriptions(body.to_owned()).await;
        assert!(
            response.status().is_client_error(),
            "invalid requests ({desc}) should lead to 40x response, get {} response message: {:?}",
            response.status().as_u16(),
            response.text().await
        );
    }
}

#[tokio::test]
async fn subscribe_return_400_for_incomplete_input() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body.to_owned()).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}
