use actix_web::{web, App, HttpServer};
use nostr::prelude::*;
use nostr_sdk::prelude::*;
use rust_blossom_server::api::upload;
use rust_blossom_server::api::AuthMiddlewareFactory;
use rust_blossom_server::blossom::action::Action;
use rust_blossom_server::config::get_config;
use rust_blossom_server::telemetry::init_tracer;
use sqlx::sqlite::SqlitePoolOptions;
use std::net::TcpListener;
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

    let listener = TcpListener::bind(format!("{}:{}", cfg.host, cfg.port))?;
    let _server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .service(
                web::resource("/upload")
                    .wrap(AuthMiddlewareFactory::new(Action::Upload))
                    .to(upload),
            )
            .app_data(data_db_pool.clone())
    })
    .listen(listener)?
    .run()
    .await?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
