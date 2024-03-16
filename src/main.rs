use actix_web::{
    cookie::{self, time::Duration},
    get, HttpResponse, Responder,
};
use hello_world::run;
use std::io::Error;

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
async fn main() -> Result<(), Error> {
    run().await
}

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
