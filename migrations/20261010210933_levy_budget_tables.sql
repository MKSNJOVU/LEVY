-- Add migration script here

CREATE TABLE IF NOT EXISTS user_goals (
id INTEGER PRIMARY KEY AUTOINCREMENT,
goal_amount BIGINT NOT NULL,
created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS user_income_categories(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    amount BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_expense_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT,
    amount BIGINT,
    is_active BIT NOT NULL DEFAULT 0
);