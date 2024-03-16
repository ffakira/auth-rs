use actix_web::{
    cookie::{self, time::Duration}, get, web, App, HttpResponse, HttpServer, Responder
};
use dotenv::dotenv;
mod handlers;
mod models;
use handlers::{auth, product};
mod db;
mod services;
use io::{Error, ErrorKind};
use mongodb::Database;
use std::{env, io};

#[derive(Clone)]
pub struct AppState {
    db: Database
}

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

    let (_, db) = db::mongo_client().await.unwrap();
    let app_state = web::Data::new(AppState { db });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(auth::configure)
            .configure(product::configure)
    })
    .bind(format!("{}:{}", host, port))?
    .run()
    .await
}

// struct CookieMiddleware;

// impl <S, B> actix_web::middleware::Middleware<S, B> for CookieMiddleware {
//     fn start(&self, req: &HttpResponse<S>, _opts: &actix_web::middleware::Started) {
//     }
// }


#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_index() {
        let mut app = test::init_service(App::new().service(index)).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status(), 200);
    }
}
