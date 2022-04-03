use chrono::prelude::*;
use primitive_types::H160;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SignUp {
    pub address: H160,
    pub signed_up_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct SignUps {
    pub total: u64,
    pub last_signed_up: Option<DateTime<Utc>>,
}
