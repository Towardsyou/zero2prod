use crate::{
    configuration::{DatabaseSettings, Settings},
    email_client::EmailClient,
    routes::{health_check, subscribe},
};
use actix_web::{dev::Server, web, App, HttpServer};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let sender = configuration
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let email_client = EmailClient::new(
            sender,
            configuration.email_client.api_url.clone(),
            configuration.email_client.authorization_token.clone(),
            configuration.email_client.timeout(),
        )
        .expect("fail to build a email client");

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address).expect("Failed to bind listener");
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, email_client)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
    // A more expressive name that makes it clear that
    // this function only returns when the application is stopped.
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(configuration: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(configuration.with_db())
}

pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
) -> Result<Server, std::io::Error> {
    let http_client = web::Data::new(email_client);
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
            .app_data(http_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
