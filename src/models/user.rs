use bcrypt::{hash, DEFAULT_COST};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{Duration, Utc};

#[derive(Serialize, Validate, Deserialize)]
pub struct User {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub is_verified: Option<bool>,
    // role_id: ObjectId,
    // organization_id: Option<ObjectId>,
    pub created_at: Option<u64>,
    pub updated_at: Option<u64>,
}

impl User {
    pub fn new(email: String, password: String) -> Result<Self, bcrypt::BcryptError> {
        let current_time = Utc::now().timestamp() as u64;
        let hash_password = hash(password, DEFAULT_COST)?;
        let trim_email = email.trim().to_lowercase();

        Ok(Self {
            email: trim_email,
            password: hash_password,
            is_verified: Some(false),
            created_at: Some(current_time),
            updated_at: Some(current_time),
        })
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct Organization {
    name: String,
    description: String,
    owner_id: ObjectId,
    created_at: Option<u64>,
    updated_at: Option<u64>,
}

pub enum RoleType {
    Admin,
    User,
}

pub enum PermissionType {
    View,
    Edit,
    Delete,
}

pub struct Role {
    name: RoleType,
    permissions: Vec<PermissionType>,
    created_at: Option<u64>,
    updated_at: Option<u64>,
}

#[derive(Serialize, Validate, Deserialize)]
pub struct Otp {
    #[validate(email)]
    pub email: String,
    pub code: u32,
    pub is_used: bool,
    pub created_at: Option<u64>,
    pub expired_at: Option<u64>,
}

impl Otp {
    pub fn new(email: String, code: u32) -> Self {
        let current_time = Utc::now().timestamp() as u64;
        let expired_at = current_time + Duration::try_minutes(10).unwrap().num_seconds() as u64;

        Self {
            email,
            code,
            is_used: false,
            created_at: Some(current_time),
            expired_at: Some(expired_at),
        }
    }
}

#[derive(Serialize, Validate, Deserialize)]
pub struct ForgotPassword {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    pub uuid: String,
    pub is_used: bool,
    pub created_at: Option<u64>,
    pub expired_at: Option<u64>,
}

#[derive(Serialize, Validate, Deserialize)]
pub struct ResendOtp {
    #[validate(email)]
    pub email: String,
}
