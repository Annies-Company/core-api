use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// What the form at #/feedback sends. Everything here is untrusted.
#[derive(Deserialize)]
pub struct FeedbackInput {
    pub rating: i32,
    pub name: String,
    #[serde(default)]
    pub area: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub ordered: Option<String>,
    pub message: String,
    // Missing means "no": a customer is only published if they said yes.
    #[serde(default)]
    pub consent: bool,
}

// The phone number decides which struct you use. There are three, so
// that "don't leak the phone" is enforced by types, not by remembering:
//
//   Feedback         — admin only. The only struct with `phone`.
//   FeedbackReceipt  — returned to whoever submitted the form. No phone.
//   PublicReview     — on the public site. Only the six fields API.md allows.

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feedback {
    pub id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub rating: i32,
    pub name: String,
    pub area: String,
    pub phone: String,
    pub ordered: String,
    pub message: String,
    pub consent: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackReceipt {
    pub id: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub rating: i32,
    pub name: String,
    pub area: String,
    pub ordered: String,
    pub message: String,
    pub consent: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicReview {
    pub name: String,
    pub area: String,
    pub rating: i32,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub photo: Option<String>,
}

#[derive(Deserialize)]
pub struct StatusUpdate {
    pub status: String,
}

pub const STATUSES: [&str; 4] = ["new", "read", "published", "hidden"];
