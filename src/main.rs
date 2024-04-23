use actix_web::{web, App, HttpServer};
use nostr::prelude::*;
use nostr_sdk::prelude::*;
use rust_blossom_server::blossom::action::Action;
use rust_blossom_server::blossom::auth::AuthMiddlewareFactory;
use rust_blossom_server::config::get_config;
use rust_blossom_server::telemetry::init_tracer;
use sqlx::sqlite::SqlitePoolOptions;
use std::net::TcpListener;
use tracing::trace;
use tracing_actix_web::TracingLogger;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().expect("failed to read env file");
    let cfg = get_config().expect("failed to read config");
    init_tracer(
        cfg.telemetry.uptrace_dsn,
        cfg.env,
        cfg.telemetry.service_name,
    )?;

    let db_pool = SqlitePoolOptions::new()
        .connect_lazy(&cfg.db.path)
        .expect("failed to create db pool");
    sqlx::migrate!().run(&db_pool).await?;
    let data_db_pool = web::Data::new(db_pool);

    let listener = TcpListener::bind("127.0.0.1:8000")?;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(AuthMiddlewareFactory::new(Action::Upload))
            .app_data(data_db_pool.clone())
    })
    .listen(listener)?
    .run()
    .await?;

    trace!("exiting, goodbye!");
    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
