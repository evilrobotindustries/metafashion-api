use chrono::prelude::*;
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct Registration {
    pub address: String,
    pub registered_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct Registrations {
    pub total: i64,
    pub last_registered: Option<DateTime<Utc>>,
}
