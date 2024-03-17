use crate::{
    models::user::{ForgotPassword, ResendOtp, User},
    AppState,
};
use crate::services::{mail, otp::Otp};
use actix_web::{
    cookie::{time::Duration as CookieDuration, Cookie},
    post, web, HttpResponse, Responder,
};
use bcrypt;
use mongodb::{bson::doc, error::Error, options::IndexOptions, Collection, Database, IndexModel};
use serde_json::json;
use validator::Validate;

#[post("/register")]
async fn register(user: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
    let user_data = user.into_inner();

    if let Err(errors) = user_data.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": errors
        }));
    }

    let user = match User::new(user_data.email, user_data.password) {
        Ok(user) => user,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to create user"
            }))
        }
    };

    match create_unique_index(&data.db).await {
        Ok(_) => {}
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to create unique index"
            }))
        }
    }

    let collection: Collection<User> = data.db.collection("users");
    match collection.insert_one(&user, None).await {
        Ok(result) => result,
        Err(_) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": "failed to insert user"
            }));
        }
    };

    let otp = match Otp::new(user.email.clone()) {
        Ok(otp) => otp,
        Err(err) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("failed to create otp: {}", err)
            }));
        }
    };

    match otp.insert_otp(&data.db).await {
        Ok(_) => {}
        Err(err) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("failed to insert otp: {}", err)
            }));
        }
    }

    match mail::send_email_confirmation(&user.email, &otp.code.to_string()).await {
        Ok(_) => {
            return HttpResponse::Ok().json(json!({
                "message": "user registered successfully",
                "otp": otp.code
            }));
        }
        Err(err) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("failed to send email: {}", err)
            }));
        }
    }
}

#[post("/verify")]
async fn verify(otp: web::Json<Otp>, data: web::Data<AppState>) -> impl Responder {
    let otp_data = otp.into_inner();

    if let Err(errors) = otp_data.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": errors
        }));
    }

    match Otp::verify_otp(otp_data.code, otp_data.email.clone(), &data.db).await {
        Ok(is_valid) => {
            if is_valid {
                let collection: Collection<User> = data.db.collection("users");
                let filter = doc! {
                    "email": otp_data.email.clone()
                };
                let update = doc! {
                    "$set": { "is_verified": true }
                };
                match collection.update_one(filter, update, None).await {
                    Ok(_) => {}
                    Err(_) => {
                        return HttpResponse::InternalServerError().json(json!({
                            "error": "failed to update user"
                        }));
                    }
                }

                return HttpResponse::Ok().json(json!({
                    "message": "OTP verified successfully"
                }));
            } else {
                return HttpResponse::BadRequest().json(json!({
                    "error": "Invalid or expired OTP"
                }));
            }
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "error": format!("Failed to verify OTP: {}", e)
            }));
        }
    }
}

#[post("/login")]
async fn login(user: web::Json<User>, data: web::Data<AppState>) -> impl Responder {
    let user_data: User = user.into_inner();
    let rust_env = if data.rust_env == "production" {
        true
    } else {
        false
    };

    if let Err(errors) = user_data.validate() {
        return HttpResponse::BadRequest().json(json!({
            "error": errors
        }));
    }

    let filter = doc! {
        "email": user_data.email.trim().to_lowercase()
    };

    let collection: Collection<User> = data.db.collection("users");
    match collection.find_one(filter, None).await {
        Ok(result) => match result {
            Some(user) => match bcrypt::verify(user_data.password, &user.password) {
                Ok(_) => {
                    let auth_cookie = Cookie::build("logged_in", "true")
                        .path("/")
                        .secure(rust_env)
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

// #[post("/resend-otp")]
// async fn resend_otp(email: web::Json<ResendOtp>, data: web::Data<AppState>) -> impl Responder {
//     let email_data = email.into_inner();
//     if let Err(errors) = email_data.validate() {
//         return HttpResponse::BadRequest().json(json!({
//             "error": errors
//         }));
//     }

//     let email = email_data.email.trim().to_lowercase();

//     let filter = doc! {
//         "email": &email_data.email,
//         "is_used": false,
//     };
//     let update = doc! {
//         "$set": { "is_used": true }
//     };
//     let otp_collection: Collection<Otp> = data.db.collection("otp");
//     match otp_collection.update_many(filter, update, None).await {
//         Ok(_) => {}
//         Err(_) => {
//             return HttpResponse::InternalServerError().json(json!({
//                 "error": "failed to update otp"
//             }))
//         }
//     }

//     match create_otp(&email, &data.db).await {
//         Ok(otp_code) => {
//             match services::mail::send_email_confirmation(&email, &otp_code.to_string()).await {
//                 Ok(_) => {
//                     return HttpResponse::Ok().json(json!({
//                         "message": "otp resent successfully",
//                         "otp": otp_code
//                     }));
//                 }
//                 Err(err) => {
//                     return HttpResponse::InternalServerError().json(json!({
//                         "error": format!("failed to send email: {}", err)
//                     }))
//                 }
//             }
//         }
//         Err(err) => {
//             return HttpResponse::InternalServerError().json(json!({
//                 "error": format!("failed to create otp: {}", err)
//             }))
//         }
//     }
// }

#[post("/logout")]
async fn logout() -> impl Responder {
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
async fn forgot_password(
    uuid: web::Path<String>,
    user: web::Json<ForgotPassword>,
) -> impl Responder {
    let _user_data = user.into_inner();

    HttpResponse::Ok().json(json!({
        "uuid": uuid.to_string(),
    }))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(register);
    cfg.service(verify);
    // cfg.service(login);
    // cfg.service(resend_otp);
    // cfg.service(logout);
    // cfg.service(forgot_password);
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
