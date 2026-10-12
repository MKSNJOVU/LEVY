use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct Account {
    pub id: i64,
    pub account_name: String,
    pub plaid_account_id: Option<String>,
    pub balance: i64,
    pub item_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub amount: i64,
    pub date: String,
    pub description: Option<String>,
    pub plaid_transaction_id: Option<String>,
    pub pending: i64,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PlaidItem {
    pub id: i64,
    pub item_id: String,
    pub access_token: String,
    pub transactions_cursor: Option<String>,
    pub environment: String,
}
