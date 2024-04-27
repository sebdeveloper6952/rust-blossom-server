use actix_cors::Cors;
use actix_web::guard;
use actix_web::{web, App, HttpServer};
use actix_web_lab::middleware::from_fn;
use nostr::prelude::*;
use nostr_sdk::prelude::*;
use rust_blossom_server::api::{
    delete, extract_payload_size_middleware, get, get_with_ext, has, has_with_ext, index_file,
    list, upload, verify_upload, PubkeyWhitelistMiddlewareFactory,
};
use rust_blossom_server::blossom::Action;
use rust_blossom_server::config::get_config;
use rust_blossom_server::telemetry::init_tracing;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::HashSet;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().expect("failed to read env file");
    let cfg = get_config().expect("failed to read config");
    let data_cfg = web::Data::new(cfg.clone());

    init_tracing(
        cfg.telemetry.uptrace_dsn,
        cfg.env,
        cfg.telemetry.service_name,
    )?;

    let db_pool = SqlitePoolOptions::new()
        .connect_lazy(&cfg.db.path)
        .expect("failed to create db pool");
    sqlx::migrate!().run(&db_pool).await?;
    let data_db_pool = web::Data::new(db_pool);

    let whitelisted_pubkeys: HashSet<&str> =
        HashSet::from(["1bbd7fdf68eaf5c19446c3aaf63b39dd4a8e33548bc96f6bd239a4124d8f229e"]);
    let data_pks = web::Data::new(whitelisted_pubkeys);

    let listener = TcpListener::bind(format!("{}:{}", cfg.host, cfg.port))?;
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allowed_methods(vec!["GET", "PUT", "HEAD", "DELETE"])
            .allowed_headers(vec!["Authorization", "Content-Type"])
            .expose_headers(vec!["Content-Length"]);

        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .route("/", web::get().to(index_file))
            .service(
                web::resource("/upload")
                    .guard(guard::Put())
                    .wrap(PubkeyWhitelistMiddlewareFactory {})
                    .wrap(from_fn(verify_upload))
                    .to(upload),
            )
            .service(
                web::resource("/delete")
                    .guard(guard::Delete())
                    .wrap(from_fn(verify_upload))
                    .to(delete),
            )
            .service(
                web::resource("/{hash}.{ext}")
                    .guard(guard::Get())
                    .to(get_with_ext),
            )
            .service(web::resource("/{hash}").guard(guard::Get()).to(get))
            .service(
                web::resource("/{hash}.{ext}")
                    .guard(guard::Head())
                    .to(has_with_ext),
            )
            .service(web::resource("/{hash}").guard(guard::Head()).to(has))
            .service(web::resource("/list/{pubkey}").guard(guard::Get()).to(list))
            .app_data(web::PayloadConfig::new(2_097_152)) // 2MB
            .app_data(data_db_pool.clone())
            .app_data(data_cfg.clone())
            .app_data(data_pks.clone())
    })
    .listen(listener)?
    .run()
    .await?;

    opentelemetry::global::shutdown_tracer_provider();
    Ok(())
}
