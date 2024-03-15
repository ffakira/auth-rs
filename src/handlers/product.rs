use actix_web::{web, HttpResponse, Responder, post};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Serialize, Validate, Deserialize)]
struct Product {
    user_id: ObjectId,
    name: String,
    description: String,
    #[validate(range(min = 0))]
    quantity: i32,
    #[validate(range(min = 0))]
    price: i64,
    category_id: Vec<ObjectId>,
    created_at: Option<u64>,
    updated_at: Option<u64>,
}

#[derive(Serialize, Deserialize)]
struct Category {
    name: String,
    created_at: Option<u64>,
    updated_at: Option<u64>,
}

#[post("/product/create")]
async fn product_create() -> impl Responder {
    HttpResponse::Ok().body("Product created")
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(product_create);
}
