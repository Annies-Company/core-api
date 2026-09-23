use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Full row from the DB — includes the hash, so it is NOT Serialize.
// That way it can never accidentally be returned as JSON.
pub struct AdminRow {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

// What the API sends back: everything except the hash.
#[derive(Serialize)]
pub struct AdminResponse {
    pub id: i64,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<AdminRow> for AdminResponse {
    fn from(row: AdminRow) -> Self {
        AdminResponse {
            id: row.id,
            email: row.email,
            created_at: row.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}
