use crate::plaid::*;
use chrono::prelude::*;
use sqlx::SqlitePool;

#[allow(dead_code)]
pub async fn handle_create_link_token(plaid_client: &PlaidClient) -> Result<String, String> {
    let response = plaid_client
        .create_link_token()
        .await?;
    
    Ok(response)
}

#[allow(dead_code)]
pub async fn handle_exchange_public_token(
    pool: &SqlitePool,
    plaid_client: &PlaidClient,
    public_token: &str,
) -> Result<(), String> {
    //  Exchange the token
    let response = plaid_client.exchange_public_token(public_token).await?;

    let access_token = response.access_token;
    let item_id = response.item_id;

    // 2. Record inception
    let current_date = Utc::now().format("%F").to_string();

    // 3. Store the item
    let _result = sqlx::query!(
        "INSERT INTO plaid_items (access_token, item_id, inception) VALUES (?,?,?)",
        access_token,
        item_id,
        current_date,
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}
