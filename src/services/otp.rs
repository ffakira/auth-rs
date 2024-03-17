use chrono::{Duration, Utc};
use mongodb::bson::doc;
use mongodb::{Collection, Database};
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use validator::Validate;

#[derive(Debug)]
pub enum OtpError {
    ValidationError(String),
    MongoError(mongodb::error::Error),
    ChronoError(String),
}

impl Display for OtpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            OtpError::ValidationError(e) => write!(f, "ValidationError: {}", e),
            OtpError::MongoError(e) => write!(f, "MongoError: {}", e),
            OtpError::ChronoError(e) => write!(f, "ChronoError: {}", e),
        }
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct Otp {
    #[validate(email)]
    pub email: String,
    pub code: u32,
    pub is_used: Option<bool>,
    pub created_at: Option<u64>,
    pub expired_at: Option<u64>,
}

impl Otp {
    fn generate_code() -> u32 {
        let mut rng = rand::thread_rng();
        return rng.gen_range(100000..=999999);
    }

    fn util_time(mins: u64) -> Result<(u64, u64), OtpError> {
        let current_time = Utc::now().timestamp() as u64;
        let expired_at =
            current_time + Duration::try_minutes(mins as i64).unwrap().num_seconds() as u64;

        if expired_at < current_time {
            return Err(OtpError::ChronoError(
                "OtpError: Expired time cannot be less than current time".to_string(),
            ));
        }

        Ok((current_time, expired_at))
    }

    pub fn new(email: String) -> Result<Self, OtpError> {
        let (current_time, expired_at) = Self::util_time(10).unwrap();
        let email = email.to_lowercase().trim().to_string();

        let otp = Otp {
            email,
            code: Self::generate_code(),
            is_used: Some(false),
            created_at: Some(current_time),
            expired_at: Some(expired_at),
        };

        match otp.validate() {
            Ok(_) => Ok(otp),
            Err(e) => Err(OtpError::ValidationError(e.to_string())),
        }
    }

    pub async fn insert_otp(&self, db: &Database) -> Result<(), OtpError> {
        let collection: Collection<Otp> = db.collection("otp");
        match collection.insert_one(self, None).await {
            Ok(_) => Ok(()),
            Err(e) => return Err(OtpError::MongoError(e)),
        }
    }

    pub async fn verify_otp(code: u32, email: String, db: &Database) -> Result<bool, OtpError> {
        let current_time = Utc::now().timestamp() as i64;
        let email = email.to_lowercase().trim().to_string();

        let collection: Collection<Otp> = db.collection("otp");
        let filter = doc! {
            "email": email.clone(),
            "code": code,
            "is_used": false,
            "expired_at": { "$gt": current_time },
        };
        let update = doc! { "$set": { "is_used": true }};

        match collection.update_one(filter, update, None).await {
            Ok(update_result) => {
                if update_result.modified_count == 1 {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => return Err(OtpError::MongoError(e)),
        }
    }

    pub async fn update_otp(email: String, db: &Database) -> Result<(String, u32), OtpError> {
        let collection: Collection<Otp> = db.collection("otp");
        let (_, expired_at) = Self::util_time(10).unwrap();
        let code = Self::generate_code();
        let filter = doc! { "email": email.clone() };
        let update = doc! {
            "$set": {
                "is_used": false,
                "code": code,
                "expired_at": expired_at as i64,
            },
        };

        match collection.update_one(filter, update, None).await {
            Ok(_) => Ok((email, code)),
            Err(e) => Err(OtpError::MongoError(e)),
        }
    }
}
