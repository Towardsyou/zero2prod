use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    email_client::EmailClient,
    issue_delivery_worker::{try_execute_task, ExecutionOutcome},
    startup::{get_connection_pool, Application},
    telemetry::{get_subscriber, init_subscriber},
};

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_confirmation_link(&self, req: &wiremock::Request) -> reqwest::Url {
        let body: serde_json::Value = serde_json::from_slice(&req.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            links[0].as_str().to_owned()
        };

        let raw_link = get_link(&body["HtmlBody"].as_str().unwrap());
        let mut confirmation_link = reqwest::Url::parse(&raw_link).expect("invalid link from resp");
        confirmation_link.set_port(Some(self.port)).unwrap();
        assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
        confirmation_link
    }

    pub async fn get_publish_newsletters(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("failed to get publish newsletters")
    }

    pub async fn get_publish_newsletters_html(&self) -> String {
        self.get_publish_newsletters().await.text().await.unwrap()
    }

    pub async fn post_publish_newsletters<Body>(&self, body: Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/newsletters", self.address))
            .form(&body)
            .send()
            .await
            .expect("failed to post publish newsletter")
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", self.address))
            .form(body)
            .send()
            .await
            .expect("failed to post login")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", self.address))
            .send()
            .await
            .expect("failed to get login html")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", self.address))
            .send()
            .await
            .expect("failed to get /admin/dashboard")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard()
            .await
            .text()
            .await
            .expect("failed to get text from admin dashboad response")
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to get change password")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password().await.text().await.unwrap()
    }

    pub async fn post_change_password<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("failed to post change password")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("failed to do admin logout")
    }

    pub async fn dispatch_all_pending_emails(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client)
                    .await
                    .unwrap()
            {
                break;
            }
        }
    }
}

pub struct TestUser {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    pub async fn login(&self, app: &TestApp) {
        app.post_login(&serde_json::json!({
            "username": self.username,
            "password": self.password,
        }))
        .await;
    }

    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // Match parameters of the default password
        let password_hash = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        sqlx::query!(
            "insert into users (user_id, username, password_hash) values ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("failed to store test user");
    }
}

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(subscriber);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server: MockServer = MockServer::start().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();

    let configuration = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = Uuid::new_v4().to_string();
        // Use a random OS port
        c.application.port = 0;
        // Use the mock email server
        c.email_client.api_url = email_server.uri();
        c
    };
    // Create and migrate the database
    configure_database(&configuration.database).await;
    // Launch the application as a background task
    let server = Application::build(configuration.clone())
        .await
        .expect("Failed to build application.");
    let port = server.port();
    let address = format!("http://127.0.0.1:{}", &port);
    let _ = tokio::spawn(server.run_until_stopped());
    let test_user = TestUser::generate();
    let pool = get_connection_pool(&configuration.database);
    test_user.store(&pool).await;
    TestApp {
        address: address,
        port: port,
        db_pool: pool,
        email_server: email_server,
        test_user: test_user,
        api_client: client,
        email_client: configuration.email_client.client().unwrap(),
    }
}

async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    // Migrate database
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

pub fn assert_is_redirect_to(resp: &reqwest::Response, location: &str) {
    assert_eq!(resp.status().as_u16(), 303);
    assert_eq!(
        resp.headers().get(reqwest::header::LOCATION).unwrap(),
        location
    );
}
