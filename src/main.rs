use actix_web::{
    cookie::{self, time::Duration},
    get, App, HttpResponse, HttpServer, Responder,
};
mod handlers;
use handlers::auth;
mod db;
mod services;

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
    let (client, _) = db::mongo_client().await.unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(client.clone())
            .service(index)
            .service(auth::register)
            .service(auth::verify)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
