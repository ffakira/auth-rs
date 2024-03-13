use actix_web::{
    cookie::{self, time::Duration},
    get, App, HttpResponse, HttpServer, Responder,
};
use dotenv::dotenv;
mod handlers;
use handlers::auth;
mod db;
mod services;
use io::{Error, ErrorKind};
use std::{env, io};

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .cookie(
            cookie::Cookie::build("logged_in", "true")
                .max_age(Duration::seconds(300))
                .finish(),
        )
        .finish()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let port = env::var("SERVER_PORT")
        .map_err(|err| Error::new(ErrorKind::Other, format!("Error loading PORT: {}", err)))
        .and_then(|port_str| {
            port_str
                .parse::<u16>()
                .map_err(|err| Error::new(ErrorKind::Other, format!("Error parsing PORT: {}", err)))
        })?;
    let host = env::var("SERVER_HOST")
        .map_err(|err| Error::new(ErrorKind::Other, format!("Error loading HOST: {}", err)))?;

    let (client, _) = db::mongo_client().await.unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(client.clone())
            .configure(auth::configure)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}
