use crate::account_structs::PlaidItem;
use crate::plaid::{PlaidAccount, PlaidClient, PlaidTransaction, SyncPage, dollars_to_cents};
use chrono::Months;
use chrono::prelude::*;
use sqlx::{Sqlite, SqlitePool, Transaction};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct SyncStats {
    pub accounts: i64,
    pub added: i64,
    pub modified: i64,
    pub removed: i64,
}

pub async fn store_item(
    pool: &SqlitePool,
    item_id: &str,
    access_token: &str,
    environment: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO plaid_items (item_id, access_token, environment)
         VALUES (?, ?, ?)
         ON CONFLICT(item_id) DO UPDATE SET
            access_token = excluded.access_token,
            environment = excluded.environment",
    )
    .bind(item_id)
    .bind(access_token)
    .bind(environment)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn sync_items(pool: &SqlitePool, client: &PlaidClient) -> Result<SyncStats, String> {
    let items: Vec<PlaidItem> = sqlx::query_as(
        "SELECT id, item_id, access_token, transactions_cursor, environment
         FROM plaid_items
         WHERE environment = ?",
    )
    .bind(client.environment())
    .fetch_all(pool)
    .await
    .map_err(|err| err.to_string())?;

    if items.is_empty() {
        println!("No Plaid items stored for this environment.");
        return Ok(SyncStats::default());
    }

    let mut stats = SyncStats::default();
    for item in items {
        let item_stats = sync_item(pool, client, &item).await?;
        stats.accounts += item_stats.accounts;
        stats.added += item_stats.added;
        stats.modified += item_stats.modified;
        stats.removed += item_stats.removed;
    }
    Ok(stats)
}

async fn sync_item(
    pool: &SqlitePool,
    client: &PlaidClient,
    item: &PlaidItem,
) -> Result<SyncStats, String> {
    let mut stats = SyncStats::default();
    let mut cursor = item.transactions_cursor.clone();
    loop {
        let page = client
            .transactions_sync(&item.access_token, cursor.as_deref())
            .await?;
        let page_stats = apply_page(pool, item.id, &page).await?;
        stats.accounts += page_stats.accounts;
        stats.added += page_stats.added;
        stats.modified += page_stats.modified;
        stats.removed += page_stats.removed;
        cursor = Some(page.next_cursor.clone());
        if !page.has_more {
            break;
        }
    }
    Ok(stats)
}

async fn apply_page(pool: &SqlitePool, item_pk: i64, page: &SyncPage) -> Result<SyncStats, String> {
    let mut tx = pool.begin().await.map_err(|err| err.to_string())?;
    let mut account_ids = HashMap::new();
    for account in &page.accounts {
        let local_id = upsert_account(&mut tx, item_pk, account).await?;
        account_ids.insert(account.account_id.clone(), local_id);
    }

    for transaction in page.added.iter().chain(page.modified.iter()) {
        let local_id = match account_ids.get(&transaction.account_id) {
            Some(id) => *id,
            None => ensure_account(&mut tx, item_pk, &transaction.account_id).await?,
        };
        account_ids.insert(transaction.account_id.clone(), local_id);
        upsert_transaction(&mut tx, local_id, transaction).await?;
    }

    for removed in &page.removed {
        sqlx::query("DELETE FROM transactions WHERE plaid_transaction_id = ?")
            .bind(&removed.transaction_id)
            .execute(&mut *tx)
            .await
            .map_err(|err| err.to_string())?;
    }

    sqlx::query("UPDATE plaid_items SET transactions_cursor = ? WHERE id = ?")
        .bind(&page.next_cursor)
        .bind(item_pk)
        .execute(&mut *tx)
        .await
        .map_err(|err| err.to_string())?;
    tx.commit().await.map_err(|err| err.to_string())?;

    Ok(SyncStats {
        accounts: page.accounts.len() as i64,
        added: page.added.len() as i64,
        modified: page.modified.len() as i64,
        removed: page.removed.len() as i64,
    })
}

async fn upsert_account(
    tx: &mut Transaction<'_, Sqlite>,
    item_pk: i64,
    account: &PlaidAccount,
) -> Result<i64, String> {
    let balance = match &account.balances.current {
        Some(amount) => dollars_to_cents(&amount.to_string())?,
        None => 0,
    };
    sqlx::query(
        "INSERT INTO accounts (account_name, plaid_account_id, balance, item_id)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(plaid_account_id) DO UPDATE SET
            account_name = excluded.account_name,
            balance = excluded.balance,
            item_id = excluded.item_id",
    )
    .bind(&account.name)
    .bind(&account.account_id)
    .bind(balance)
    .bind(item_pk)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    account_row_id(tx, &account.account_id).await
}

async fn ensure_account(
    tx: &mut Transaction<'_, Sqlite>,
    item_pk: i64,
    plaid_account_id: &str,
) -> Result<i64, String> {
    sqlx::query(
        "INSERT INTO accounts (account_name, plaid_account_id, balance, item_id)
         VALUES ('Linked account', ?, 0, ?)
         ON CONFLICT(plaid_account_id) DO UPDATE SET
            item_id = excluded.item_id",
    )
    .bind(plaid_account_id)
    .bind(item_pk)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    account_row_id(tx, plaid_account_id).await
}

async fn account_row_id(
    tx: &mut Transaction<'_, Sqlite>,
    plaid_account_id: &str,
) -> Result<i64, String> {
    let (id,): (i64,) = sqlx::query_as("SELECT id FROM accounts WHERE plaid_account_id = ?")
        .bind(plaid_account_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|err| err.to_string())?;
    Ok(id)
}

async fn upsert_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    account_id: i64,
    transaction: &PlaidTransaction,
) -> Result<(), String> {
    let amount = dollars_to_cents(&transaction.amount.to_string())?;
    let pending: i64 = if transaction.pending { 1 } else { 0 };
    let category = transaction
        .personal_finance_category
        .as_ref()
        .map(|category| category.primary.as_str());
    sqlx::query(
        "INSERT INTO transactions
            (account_id, amount, date, description, plaid_transaction_id, pending, category)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(plaid_transaction_id) DO UPDATE SET
            account_id = excluded.account_id,
            amount = excluded.amount,
            date = excluded.date,
            description = excluded.description,
            pending = excluded.pending,
            category = excluded.category",
    )
    .bind(account_id)
    .bind(amount)
    .bind(&transaction.date)
    .bind(&transaction.name)
    .bind(&transaction.transaction_id)
    .bind(pending)
    .bind(category)
    .execute(&mut **tx)
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

/// Shell for executing the transaction sync with strict time boundaries
#[allow(dead_code)]
pub async fn sync_and_prune_transactions(
    pool: &SqlitePool,
    plaid_client: &PlaidClient,
    access_token: &str,
    inception_date: &str, // Passed in from your DB query of the plaid_items table
) -> Result<(), String> {
    let mut cursor: Option<String> = None;
    //Fetch from Plaid

    loop {
        let transactions = plaid_client
            .transactions_sync(access_token, cursor.as_deref())
            .await?;

        for tx in &transactions.added {
            if tx.pending || tx.date.as_str() < inception_date {
                continue;
            }
            let amount_cents = dollars_to_cents(&tx.amount.to_string()).unwrap_or(0);
            sqlx::query!(
                r#"
                INSERT OR IGNORE INTO transactions (
                    account_id, amount, date, description, plaid_transaction_id, pending
                ) VALUES (?, ?, ?, ?, ?, 0)
                "#,
                tx.account_id,
                amount_cents,
                tx.date,
                tx.name,
                tx.transaction_id,
            )
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }

        for tx in &transactions.modified {
            if tx.pending || tx.date.as_str() < inception_date {
                continue;
            }
            let amount_cents = dollars_to_cents(&tx.amount.to_string()).unwrap_or(0);
            sqlx::query!(
                r#"
                UPDATE transactions
                SET account_id = ?, amount = ?, date = ?, description = ?
                WHERE plaid_transaction_id = ?
                "#,
                tx.account_id,
                amount_cents,
                tx.date,
                tx.name,
                tx.transaction_id,
            )
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }

        for tx in &transactions.removed {
            sqlx::query!(
                "DELETE FROM transactions WHERE plaid_transaction_id = ?",
                tx.transaction_id
            )
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        }

        if !transactions.has_more {
            break;
        } else {
            cursor = Some(transactions.next_cursor);
        }
    }

    //  Prune Old Data
    let cutoff_date = Local::now()
        .date_naive()
        .checked_sub_months(Months::new(24))
        .unwrap()
        .format("%Y-%m")
        .to_string();

    let _old_data = sqlx::query!(
        "DELETE from monthly_summaries WHERE month_year < (?)",
        cutoff_date,
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query!(
        "DELETE FROM transactions WHERE substr(date, 1, 7) < ?",
        cutoff_date
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use crate::account_structs::{Account, Transaction};
    use crate::plaid::{PlaidClient, PlaidConfig};

    use super::{store_item, sync_items};

    #[tokio::test]
    async fn sandbox_exchange_and_second_sync_are_idempotent() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&requests);
        tokio::spawn(async move {
            run_sandbox(listener, counter).await;
        });

        let path = std::env::temp_dir().join(format!(
            "levy-plaid-sync-{}-{}.db",
            std::process::id(),
            addr.port()
        ));
        let _ = std::fs::remove_file(&path);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite://{}?mode=rwc", path.display()))
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let client = PlaidClient::new(PlaidConfig {
            base_url: format!("http://{addr}"),
            environment: "sandbox".to_string(),
            client_id: "client-id".to_string(),
            secret: "secret".to_string(),
            version: "2020-09-14".to_string(),
        })
        .unwrap();

        let public_token = client.create_sandbox_public_token().await.unwrap();
        let credentials = client.exchange_public_token(&public_token).await.unwrap();
        assert_eq!(credentials.item_id, "item-sandbox");
        store_item(
            &pool,
            &credentials.item_id,
            &credentials.access_token,
            client.environment(),
        )
        .await
        .unwrap();

        let first = sync_items(&pool, &client).await.unwrap();
        assert_eq!(first.added, 2);
        assert_eq!(first.accounts, 4);

        let accounts_after_first = load_accounts(&pool).await;
        let transactions_after_first = load_transactions(&pool).await;
        assert_eq!(accounts_after_first.len(), 2);
        assert_eq!(transactions_after_first.len(), 2);

        let second = sync_items(&pool, &client).await.unwrap();
        assert_eq!(second.added, 2);
        assert_eq!(second.modified, 1);

        let accounts_after_second = load_accounts(&pool).await;
        let transactions_after_second = load_transactions(&pool).await;
        assert_eq!(
            accounts_after_second
                .iter()
                .map(|account| account.id)
                .collect::<Vec<_>>(),
            accounts_after_first
                .iter()
                .map(|account| account.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            transactions_after_second
                .iter()
                .map(|transaction| transaction.id)
                .collect::<Vec<_>>(),
            transactions_after_first
                .iter()
                .map(|transaction| transaction.id)
                .collect::<Vec<_>>()
        );
        assert_eq!(transactions_after_second.len(), 2);

        let checking = transactions_after_second
            .iter()
            .find(|transaction| {
                transaction.plaid_transaction_id.as_deref() == Some("txn-checking-1")
            })
            .unwrap();
        let card = transactions_after_second
            .iter()
            .find(|transaction| transaction.plaid_transaction_id.as_deref() == Some("txn-card-1"))
            .unwrap();
        assert_eq!(checking.amount, 1500);
        assert_eq!(checking.pending, 0);
        assert_eq!(card.amount, -501);
        assert_eq!(card.pending, 1);

        let checking_account = accounts_after_second
            .iter()
            .find(|account| account.plaid_account_id.as_deref() == Some("acc-checking"))
            .unwrap();
        let card_account = accounts_after_second
            .iter()
            .find(|account| account.plaid_account_id.as_deref() == Some("acc-credit"))
            .unwrap();
        assert_eq!(checking_account.account_name, "Plaid Checking");
        assert_eq!(checking_account.balance, 11050);
        assert_eq!(card_account.account_name, "Plaid Credit Card");
        assert_eq!(card_account.balance, 2510);
        assert!(checking_account.item_id.is_some());

        let _ = std::fs::remove_file(&path);
        assert!(requests.load(Ordering::SeqCst) >= 4);
    }

    async fn load_accounts(pool: &sqlx::SqlitePool) -> Vec<Account> {
        sqlx::query_as(
            "SELECT id, account_name, plaid_account_id, balance, item_id FROM accounts ORDER BY id",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn load_transactions(pool: &sqlx::SqlitePool) -> Vec<Transaction> {
        sqlx::query_as(
            "SELECT id, account_id, amount, date, description, plaid_transaction_id, pending, category
             FROM transactions
             ORDER BY id",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    async fn run_sandbox(listener: TcpListener, requests: Arc<AtomicUsize>) {
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                break;
            };
            let requests = Arc::clone(&requests);
            tokio::spawn(async move {
                loop {
                    let Some((path, body)) = read_http(&mut socket).await else {
                        break;
                    };
                    requests.fetch_add(1, Ordering::SeqCst);
                    let response = match path.as_str() {
                        "/sandbox/public_token/create" => {
                            json!({ "public_token": "public-sandbox", "request_id": "req-public" })
                                .to_string()
                        }
                        "/item/public_token/exchange" => json!({
                            "access_token": "access-sandbox",
                            "item_id": "item-sandbox",
                            "request_id": "req-exchange"
                        })
                        .to_string(),
                        "/transactions/sync" => sync_body(&body),
                        _ => json!({ "error_code": "UNKNOWN", "error_message": "not found" })
                            .to_string(),
                    };
                    let payload = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n{response}",
                        response.len()
                    );
                    if socket.write_all(payload.as_bytes()).await.is_err() {
                        break;
                    }
                }
            });
        }
    }

    fn sync_body(body: &str) -> String {
        let cursor = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| {
                value
                    .get("cursor")
                    .and_then(|cursor| cursor.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default();
        let accounts = serde_json::from_str::<serde_json::Value>(
            r#"[{"account_id":"acc-checking","name":"Plaid Checking","balances":{"current":110.5}},{"account_id":"acc-credit","name":"Plaid Credit Card","balances":{"current":25.10}}]"#,
        )
        .unwrap();
        let checking = serde_json::from_str::<serde_json::Value>(
            r#"{"transaction_id":"txn-checking-1","account_id":"acc-checking","amount":12.34,"date":"2026-10-01","name":"Coffee","pending":false}"#,
        )
        .unwrap();
        let card = serde_json::from_str::<serde_json::Value>(
            r#"{"transaction_id":"txn-card-1","account_id":"acc-credit","amount":-5.005,"date":"2026-10-02","name":"Refund","pending":true}"#,
        )
        .unwrap();
        let (added, modified, next_cursor, has_more) = if cursor.is_empty() {
            (json!([checking]), json!([]), "cursor-1", true)
        } else if cursor == "cursor-1" {
            (json!([card]), json!([]), "cursor-2", false)
        } else {
            let modified_checking = serde_json::from_str::<serde_json::Value>(
                r#"{"transaction_id":"txn-checking-1","account_id":"acc-checking","amount":15.00,"date":"2026-10-01","name":"Coffee","pending":false}"#,
            )
            .unwrap();
            (
                json!([checking, card]),
                json!([modified_checking]),
                "cursor-2",
                false,
            )
        };
        json!({
            "added": added,
            "modified": modified,
            "removed": [],
            "next_cursor": next_cursor,
            "has_more": has_more,
            "accounts": accounts
        })
        .to_string()
    }

    async fn read_http(socket: &mut tokio::net::TcpStream) -> Option<(String, String)> {
        let mut buf = Vec::new();
        let mut tmp = [0u8; 2048];
        let header_end = loop {
            let n = socket.read(&mut tmp).await.ok()?;
            if n == 0 {
                return None;
            }
            buf.extend_from_slice(&tmp[..n]);
            if let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n") {
                break pos + 4;
            }
            if buf.len() > 1_000_000 {
                return None;
            }
        };
        let header = String::from_utf8_lossy(&buf[..header_end]).to_string();
        let content_length = header
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        while buf.len() < header_end + content_length {
            let n = socket.read(&mut tmp).await.ok()?;
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let path = header
            .lines()
            .next()?
            .split_whitespace()
            .nth(1)?
            .to_string();
        let body_end = (header_end + content_length).min(buf.len());
        let body = String::from_utf8_lossy(&buf[header_end..body_end]).to_string();
        Some((path, body))
    }
}
