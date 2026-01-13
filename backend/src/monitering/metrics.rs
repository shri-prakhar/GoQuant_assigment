use actix_web::{HttpResponse, Responder};
use once_cell::sync::Lazy;
use prometheus::{Counter, Encoder, Gauge, Registry, TextEncoder};

static REGISTRY: Lazy<Registry> = Lazy::new(|| Registry::new());
static VAULT_COUNT: Lazy<Gauge> = Lazy::new(|| {
    let guage = Gauge::new("vault_total_count", "Total Number Of Count").unwrap();
    REGISTRY.register(Box::new(guage.clone())).unwrap();
    guage
});
static TVL: Lazy<Gauge> = Lazy::new(|| {
    let gauge = Gauge::new("vault_tvl", "Total Value Locked").unwrap();
    REGISTRY.register(Box::new(gauge.clone())).unwrap();
    gauge
});
static API_REQUESTS: Lazy<Counter> = Lazy::new(|| {
    let counter = Counter::new("api_requests_total", "Total API Requests").unwrap();
    REGISTRY.register(Box::new(counter.clone())).unwrap();
    counter
});

pub fn increament_api_requests() {
    API_REQUESTS.inc();
}
pub fn set_vault_count(count: f64) {
    VAULT_COUNT.set(count);
}
pub fn set_tvl(tvl: f64) {
    TVL.set(tvl);
}
pub async fn metrics() -> impl Responder {
    let encoder = TextEncoder::new();
    let metrics_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metrics_families, &mut buffer).unwrap();

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(buffer)
}
