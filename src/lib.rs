use actix_web::{web, App, HttpServer};
use dotenvy::dotenv;
use io::{Error, ErrorKind};
use mongodb::Database;
use std::{env, io};

pub mod db;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;

fn load_server_env() -> Result<(u16, String, String), Error> {
    dotenv().ok();
    let port = env::var("SERVER_PORT")
        .map_err(|err| Error::new(ErrorKind::Other, format!("Error loading PORT: {}", err)))
        .and_then(|port_str| {
            port_str
                .parse::<u16>()
                .map_err(|err| Error::new(ErrorKind::Other, format!("Error parsing PORT: {}", err)))
        })?;
    let host = env::var("SERVER_HOST").map_err(|err| {
        Error::new(
            ErrorKind::Other,
            format!("Error loading SERVER_HOST: {}", err),
        )
    });
    let rust_env = env::var("RUST_ENV").unwrap_or("development".to_string());

    Ok((port, host.unwrap(), rust_env))
}

#[derive(Clone)]
pub struct AppState {
    db: Database,
    rust_env: String,
}

pub async fn run() -> Result<(), Error> {
    let (port, host, rust_env) = load_server_env().unwrap();
    let (_, db) = db::mongo_client().await.unwrap();
    let app_state = web::Data::new(AppState { db, rust_env });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::auth::Auth)
            .configure(handlers::auth::configure)
            .configure(handlers::product::configure)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
