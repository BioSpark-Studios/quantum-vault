use myth_wire::new_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseRecord {
    pub id: String,
    pub user_id: String,
    pub blueprint_id: String,
    pub credits_spent: u32,
    pub purchased_at: i64,
}

impl PurchaseRecord {
    pub fn new(user_id: impl Into<String>, blueprint_id: impl Into<String>, credits: u32) -> Self {
        PurchaseRecord {
            id: new_id(),
            user_id: user_id.into(),
            blueprint_id: blueprint_id.into(),
            credits_spent: credits,
            purchased_at: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub record: PurchaseRecord,
    pub remaining_credits: u32,
}

#[derive(Debug, Serialize)]
pub struct CheckoutError {
    pub error: String,
}
