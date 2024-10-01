use actix_web::{HttpResponse, Responder};
use uuid::Uuid;

pub async fn health_check() -> impl Responder {
    let request_id = Uuid::new_v4();

    // let request_span =
    //     tracing::info_span!("health check", %request_id, test_key="key", test_value="value");
    // let _request_span_guard = request_span.enter();
    // tracing::info!("before health check");
    // tracing::info!("after health check");
    log::info!("Hello, log from health check {}", request_id);
    HttpResponse::Ok()
}
