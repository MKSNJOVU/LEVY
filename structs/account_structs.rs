use std::mem::type_info::Str;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Account{
    pub id: i64,
    pub account_name: String,
    pub plaid_account_id: Option<String>,
    pub balance: i64
}
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Transaction{
    pub id: i64,
    pub account_id: i64,
    pub amount: i64,
    pub date: String,
    pub description: Option<String>,
}


