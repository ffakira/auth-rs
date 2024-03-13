use crate::{db, services};
use actix_web::{
    cookie::{time::Duration as CookieDuration, Cookie},
    post, web, HttpResponse, Responder,
};
use bcrypt::{hash, DEFAULT_COST};
use chrono::{Duration, Utc};
use mongodb::{bson::doc, error::Error, options::IndexOptions, Collection, Database, IndexModel};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

#[derive(Serialize, Validate, Deserialize)]
struct User {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
    is_verified: Option<bool>,
    created_at: Option<u64>,
    updated_at: Option<u64>,
}

#[derive(Serialize, Validate, Deserialize)]
struct Otp {
    #[validate(email)]
    email: String,
    code: u32,
    is_used: bool,
    created_at: Option<u64>,
    expired_at: Option<u64>,
}

#[derive(Serialize, Validate, Deserialize)]
struct ForgotPassword {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
    uuid: String,
    is_used: bool,
    created_at: Option<u64>,
    expired_at: Option<u64>,
}

#[post("/register")]
pub async fn register(user: web::Json<User>) -> impl Responder {
    let mut user_data = user.into_inner();

    if let Err(errors) = user_data.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": errors
        }));
    }

    // Hash password
    let hashed_password = match hash(user_data.password, DEFAULT_COST) {
        Ok(password) => password,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to hash password"
            }))
        }
    };

    let email = user_data.email.trim().to_lowercase().clone();
    // Set fields
    user_data = User {
        password: hashed_password,
        email: user_data.email.trim().to_lowercase(),
        is_verified: Some(false),
        created_at: Some(Utc::now().timestamp() as u64),
        updated_at: Some(Utc::now().timestamp() as u64),
    };

    // Insert to database
    let (_, db) = match db::mongo_client().await {
        Ok(client_db) => client_db,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to connect to database"
            }))
        }
    };

    match create_unique_index(&db).await {
        Ok(_) => {}
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to create unique index"
            }))
        }
    }

    let collection: Collection<User> = db.collection("users");
    match collection.insert_one(user_data, None).await {
        Ok(result) => result,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to insert user"
            }));
        }
    };

    match create_otp(&email).await {
        Ok(otp_code) => {
            match services::mail::send_email_confirmation(&email, &otp_code.to_string()).await {
                Ok(_) => {
                    return HttpResponse::Ok().json(json!({
                        "message": "user registered successfully",
                        "otp": otp_code
                    }));
                }
                Err(err) => {
                    return HttpResponse::InternalServerError().json(json!({
                        "error": format!("failed to send email: {}", err)
                    }))
                }
            }
        }
        Err(err) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("failed to create otp: {}", err)
            }))
        }
    }
}

#[post("/verify")]
pub async fn verify(otp: web::Json<Otp>) -> impl Responder {
    let otp_data = otp.into_inner();
    let current_time = Utc::now().timestamp() as i64;
    let filter = doc! {
        "email": &otp_data.email,
        "is_used": false,
        "expired_at": { "$lt": current_time }
    };
    let (_, db) = match db::mongo_client().await {
        Ok(client_db) => client_db,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to connect to database"
            }))
        }
    };
    let otp_collection: Collection<Otp> = db.collection("otp");
    match otp_collection.find_one(filter, None).await {
        Ok(result) => {
            match result {
                Some(_) => {
                    let update_filter = doc! {
                        "email": &otp_data.email,
                        "is_used": false,
                        "expired_at": { "$lt": current_time }
                    };

                    let update_data = doc! {
                        "$set": { "is_used": true }
                    };

                    if let Err(_) = otp_collection
                        .update_one(update_filter, update_data, None)
                        .await
                    {
                        return HttpResponse::InternalServerError().json(json!({
                            "error": "failed to update otp"
                        }));
                    }

                    let user_filter = doc! {
                        "email": &otp_data.email,
                    };
                    let user_update = doc! {
                        "$set": { "is_verified": true }
                    };
                    let user_collection: Collection<User> = db.collection("users");
                    match user_collection
                        .find_one_and_update(user_filter, user_update, None)
                        .await
                    {
                        Ok(_) => {
                            // If user verified successfully, return success response
                            return HttpResponse::Ok().json(json!({
                                "message": "user verified successfully"
                            }));
                        }
                        Err(_) => {
                            // If failed to update user, return error response
                            return HttpResponse::InternalServerError().json(json!({
                                "error": "failed to update user"
                            }));
                        }
                    }
                }
                None => {
                    // If OTP not found, return error response
                    return HttpResponse::BadRequest().json(json!({
                        "error": "otp not found"
                    }));
                }
            }
        }
        Err(_) => {
            return HttpResponse::NotFound().json(json!({
                "error": "failed to find otp"
            }))
        }
    }
}

#[post("/login")]
pub async fn login(user: web::Json<User>) -> impl Responder {
    let user_data: User = user.into_inner();

    if let Err(errors) = user_data.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": errors
        }));
    }

    let filter = doc! {
        "email": user_data.email.trim().to_lowercase()
    };

    let (_, db) = match db::mongo_client().await {
        Ok(client_db) => client_db,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to connect to database"
            }))
        }
    };

    let collection: Collection<User> = db.collection("users");
    match collection.find_one(filter, None).await {
        Ok(result) => match result {
            Some(user) => match bcrypt::verify(user_data.password, &user.password) {
                Ok(_) => {
                    let auth_cookie = Cookie::build("logged_in", "true")
                        .path("/")
                        .secure(false)
                        .http_only(true)
                        .max_age(CookieDuration::minutes(10))
                        .finish();
                    return HttpResponse::Ok().cookie(auth_cookie).json(json!({
                        "message": "user logged in successfully"
                    }));
                }
                Err(_) => {
                    return HttpResponse::BadRequest().json(json!({
                        "error": "invalid password"
                    }))
                }
            },
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "user not found"
                }))
            }
        },
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to find user"
            }))
        }
    }
}

#[post("/logout")]
pub async fn logout() -> impl Responder {
    let cookie = Cookie::build("logged_in", "")
        .path("/")
        .max_age(CookieDuration::seconds(0))
        .finish();
    let mut response = HttpResponse::Ok().json(json!({
        "message": "user logged out successfully"
    }));
    if let Err(_) = response.add_cookie(&cookie) {
        return HttpResponse::InternalServerError().json(json!({
            "error": "failed to add cookie"
        }));
    }
    response
}

#[post("/forgot-password/{uuid}")]
pub async fn forgot_password(
    uuid: web::Path<String>,
    user: web::Json<ForgotPassword>,
) -> impl Responder {
    let _user_data = user.into_inner();

    HttpResponse::Ok().json(json!({
        "uuid": uuid.to_string(),
    }))
}

async fn create_unique_index(db: &Database) -> Result<(), Error> {
    let collection: Collection<User> = db.collection("users");
    let model = IndexModel::builder()
        .keys(doc! { "email": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();
    collection.create_index(model, None).await?;
    Ok(())
}

async fn create_otp(email: &str) -> Result<u32, Error> {
    let (_, db) = match db::mongo_client().await {
        Ok(client_db) => client_db,
        Err(e) => return Err(e),
    };
    let collection: Collection<Otp> = db.collection("otp");

    let mut rng = rand::thread_rng();
    let otp_code: u32 = rng.gen_range(100000..=999999);

    let current_time = Utc::now().timestamp() as u64;
    let expired_at = current_time + Duration::try_minutes(10).unwrap().num_seconds() as u64;

    let otp_data = Otp {
        email: email.to_string(),
        code: otp_code,
        is_used: false,
        created_at: Some(Utc::now().timestamp() as u64),
        expired_at: Some(expired_at),
    };

    match collection.insert_one(otp_data, None).await {
        Ok(_) => Ok(otp_code),
        Err(e) => Err(e),
    }
}
