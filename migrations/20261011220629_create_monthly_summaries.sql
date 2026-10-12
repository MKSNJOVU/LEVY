-- Add migration script here
-- Store the top-level month math
CREATE TABLE IF NOT EXISTS monthly_summaries (
                                                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                                                 month_year TEXT NOT NULL UNIQUE,
                                                 income BIGINT NOT NULL,
                                                 fixed_expenses BIGINT NOT NULL,
                                                 varying_expenses BIGINT NOT NULL,
                                                 goal_amount BIGINT NOT NULL,
                                                 money_medium BIGINT NOT NULL,
                                                 miss_amount BIGINT NOT NULL,
                                                 created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Store the individual varying categories (e.g., Food = 40000)
CREATE TABLE IF NOT EXISTS monthly_category_buckets (
                                                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                                                        summary_id INTEGER NOT NULL,
                                                        category_name TEXT NOT NULL,
                                                        amount BIGINT NOT NULL,
                                                        FOREIGN KEY(summary_id) REFERENCES monthly_summaries(id) ON DELETE CASCADE
    );