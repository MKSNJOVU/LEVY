mod plaid;
mod sync;
#[path = "../structs/money_medium_structs.rs"]
pub mod money_medium_structs;
#[path = "../structs/account_structs.rs"]
pub mod account_structs;

#[path = "../traits/money_medium_traits.rs"]
mod money_medium_traits;

use sqlx::sqlite::SqlitePoolOptions;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://LEVY.db".to_string());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => print_status(&pool).await?,
        Some("link") => {
            let client = plaid::PlaidClient::from_env()?;
            let token = client.create_link_token().await?;
            println!("{token}");
        }
        Some("exchange") => {
            let Some(public_token) = args.next() else {
                eprintln!("usage: levy exchange <public_token>");
                std::process::exit(2);
            };
            let client = plaid::PlaidClient::from_env()?;
            let item = client.exchange_public_token(&public_token).await?;
            sync::store_item(
                &pool,
                &item.item_id,
                &item.access_token,
                client.environment(),
            )
            .await?;
            println!("Stored Plaid item {}.", item.item_id);
        }
        Some("sync") => {
            let client = plaid::PlaidClient::from_env()?;
            let stats = sync::sync_items(&pool, &client).await?;
            println!(
                "Sync finished: {} account updates, {} added, {} modified, {} removed.",
                stats.accounts, stats.added, stats.modified, stats.removed
            );
        }
        Some(_) => {
            eprintln!("Usage: levy [link | exchange <public_token> | sync]");
            std::process::exit(2);
        }
    }

    Ok(())
}

async fn print_status(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    println!("Successfully connected to SQLite!");
    let row: (i64,) = sqlx::query_as("SELECT count(*) FROM accounts")
        .fetch_one(pool)
        .await?;
    println!("Found {} accounts.", row.0);
    Ok(())
}
