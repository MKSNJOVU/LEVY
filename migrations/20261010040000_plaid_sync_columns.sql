CREATE TABLE plaid_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id TEXT NOT NULL UNIQUE,
    access_token TEXT NOT NULL,
    transactions_cursor TEXT,
    environment TEXT NOT NULL
);

ALTER TABLE accounts ADD COLUMN item_id INTEGER REFERENCES plaid_items (id);

CREATE UNIQUE INDEX accounts_plaid_account_id_key ON accounts (plaid_account_id);

ALTER TABLE transactions ADD COLUMN plaid_transaction_id TEXT;

ALTER TABLE transactions ADD COLUMN pending INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX transactions_plaid_transaction_id_key ON transactions (plaid_transaction_id);
