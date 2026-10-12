use crate::money_medium_structs::*;
use crate::plaid::*;
use sqlx::SqlitePool;
pub trait ILevyOrchestration: IUserSavingsGoal + ICalculateMoneyMedium {}

pub trait IUserSavingsGoal {
    async fn log_user_savings_goal(&self, pool: &SqlitePool) -> Result<i64, sqlx::Error>;
}

pub trait ICalculateMoneyMedium {
    async fn user_income(&self) -> i64;
    async fn user_expense(&self) -> i64;
    async fn disposable_income(&self) -> i64 {
        let income = self.user_income().await;
        let expenses = self.user_expense().await;
        income - expenses
    }
}

// Implementations
impl ICalculateMoneyMedium for UserMoneyMedium {
    async fn user_income(&self) -> i64 {
        self.user_income.monthly_income
    }
    async fn user_expense(&self) -> i64 {
        let total_fixed_costs: i64 = self
            .user_expenses
            .fixed_expenses
            .iter()
            .map(|category| category.amount.unwrap_or(0))
            .sum();

        total_fixed_costs
    }
}

impl IUserSavingsGoal for UserMoneyMedium {
    async fn log_user_savings_goal(&self, pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        let _result = sqlx::query!(
            "INSERT INTO user_goals (goal_amount) VALUES (?)",
            self.user_goal
        )
        .execute(pool)
        .await?;

        Ok(self.user_goal)
    }
}

impl ILevyOrchestration for UserMoneyMedium {}

impl UserMoneyMedium {
    pub fn process_plaid_transactions(&mut self, transactions: Vec<PlaidTransaction>) {
        for tx in transactions {
            if tx.pending == true {
                continue;
            }

            let amount_in_cents = dollars_to_cents(&tx.amount.to_string()).unwrap_or(0);

            if let Some(pfc) = &tx.personal_finance_category {
                match pfc.primary.as_str() {
                    "INCOME" => {
                        // Plaid amounts are negative for inflows, so you may need to invert the sign
                        let inverted_amount = -amount_in_cents;
                        self.user_income
                            .current_earnings
                            .push(Some(inverted_amount));

                        self.user_income.monthly_income += inverted_amount
                    }

                    // --- FIXED EXPENSES ---
                    "RENT_AND_UTILITIES" | "LOAN_PAYMENTS" | "INSURANCE" => {
                        let category_name = pfc.primary.clone();
                        if let Some(existing_category) = self
                            .user_expenses
                            .fixed_expenses
                            .iter_mut()
                            .find(|c| c.name == category_name)
                        {
                            let current_total = existing_category.amount.unwrap_or(0);
                            existing_category.amount = Some(current_total + amount_in_cents);
                        } else {
                            let new_category = ExpenseCategory {
                                name: category_name,
                                description: Some(pfc.detailed.clone()),
                                amount: Some(amount_in_cents),
                                is_active: true,
                            };
                            self.user_expenses.fixed_expenses.push(new_category);
                        }
                    }

                    // --- VARYING EXPENSES ---
                    "FOOD_AND_DRINK"
                    | "TRANSPORTATION"
                    | "GENERAL_MERCHANDISE"
                    | "ENTERTAINMENT" => {
                        let category_name = pfc.primary.clone();
                        let tx_name = tx.name.as_deref().unwrap_or("").to_lowercase();

                        if tx_name.contains("ai") {
                            if let Some(existing) = self
                                .user_expenses
                                .varying_expenses
                                .iter_mut()
                                .find(|c| c.name == "AI Tooling")
                            {
                                let current = existing.amount.unwrap_or(0);
                                existing.amount = Some(current + amount_in_cents);
                            } else {
                                self.user_expenses.varying_expenses.push(ExpenseCategory {
                                    name: "AI".to_string(),
                                    description: Some("AI Tooling and Learning ".to_string()),
                                    amount: Some(amount_in_cents),
                                    is_active: true,
                                });
                            }
                        } else {
                            if let Some(existing_category) = self
                                .user_expenses
                                .varying_expenses
                                .iter_mut()
                                .find(|c| c.name == category_name)
                            {
                                let current_total = existing_category.amount.unwrap_or(0);
                                existing_category.amount = Some(current_total + amount_in_cents);
                            } else {
                                let new_category = ExpenseCategory {
                                    name: category_name,
                                    description: Some(pfc.detailed.clone()),
                                    amount: Some(amount_in_cents),
                                    is_active: true,
                                };
                                self.user_expenses.varying_expenses.push(new_category);
                            }
                        }
                    }

                    // --- TRANSFERS ---
                    "TRANSFER_IN" | "TRANSFER_OUT" => {
                        let tx_name = tx.name.as_deref().unwrap_or("").to_lowercase();

                        if tx_name.contains("sendwave") {
                            if let Some(existing) = self
                                .user_expenses
                                .varying_expenses
                                .iter_mut()
                                .find(|c| c.name == "Sendwave")
                            {
                                let current = existing.amount.unwrap_or(0);
                                existing.amount = Some(current + amount_in_cents);
                            } else {
                                self.user_expenses.varying_expenses.push(ExpenseCategory {
                                    name: "Remittances".to_string(),
                                    description: Some("Sendwave Transfer".to_string()),
                                    amount: Some(amount_in_cents),
                                    is_active: true,
                                });
                            }
                        } else {
                            match pfc.detailed.as_str() {
                                "TRANSFER_OUT_ACCOUNT_TRANSFER"
                                | "TRANSFER_IN_ACCOUNT_TRANSFER" => continue,
                                "TRANSFER_OUT_WIRE" | "TRANSFER_OUT_REMITTANCE" => {
                                    if let Some(existing) = self
                                        .user_expenses
                                        .varying_expenses
                                        .iter_mut()
                                        .find(|c| c.name == "Wire Transfers")
                                    {
                                        let current = existing.amount.unwrap_or(0);
                                        existing.amount = Some(current + amount_in_cents)
                                    } else {
                                        self.user_expenses.varying_expenses.push(ExpenseCategory {
                                            name: "Wire Transfers".to_string(),
                                            description: Some(pfc.detailed.clone()),
                                            amount: Some(amount_in_cents),
                                            is_active: true,
                                        });
                                    }
                                }
                                _ => {
                                    if let Some(existing) = self
                                        .user_expenses
                                        .varying_expenses
                                        .iter_mut()
                                        .find(|c| c.name == "External Transfers")
                                    {
                                        let current = existing.amount.unwrap_or(0);
                                        existing.amount = Some(current + amount_in_cents)
                                    } else {
                                        self.user_expenses.varying_expenses.push(ExpenseCategory {
                                            name: "External Transfers".to_string(),
                                            description: Some(pfc.detailed.clone()),
                                            amount: Some(amount_in_cents),
                                            is_active: true,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    // --- CATCH-ALL ---
                    _ => {
                        self.user_expenses.varying_expenses.push(ExpenseCategory {
                            name: "Wire Transfers".to_string(),
                            description: Some(pfc.detailed.clone()),
                            amount: Some(amount_in_cents),
                            is_active: true,
                        });
                    }
                }
            } else {
                // todo!("Handle legacy transactions that do not have a personal_finance_category attached")
            }
        }
    }

    /// Calculates the monthly totals from SQLite and upserts the summary and buckets.
    pub async fn close_out_month(
        &self,
        pool: &SqlitePool,
        target_month: &str,
    ) -> Result<(), sqlx::Error> {
        // Posted charges for this calendar month. Dates are stored as YYYY-MM-DD.
        let transactions = sqlx::query!(
            r#"
            SELECT
                id,
                account_id,
                amount,
                date,
                description,
                plaid_transaction_id,
                pending,
                category
            FROM transactions
            WHERE substr(date, 1, 7) = ?
              AND pending = 0
            "#,
            target_month
        )
        .fetch_all(pool)
        .await?;

        // Amounts in transactions are already integer cents.
        let mut income: i64 = 0;
        let mut fixed_expense: i64 = 0;
        let mut varying_buckets: std::collections::BTreeMap<String, i64> =
            std::collections::BTreeMap::new();

        for item in transactions {
            let amount = item.amount;
            let category_name = item.category.unwrap_or_default();
            match category_name.as_str() {
                "INCOME" => income += amount.abs(),
                "RENT_AND_UTILITIES" | "LOAN_PAYMENTS" | "INSURANCE" => fixed_expense += amount,
                _ => {
                    varying_buckets
                        .entry(category_name)
                        .and_modify(|total| *total += amount)
                        .or_insert(amount);
                }
            }
        }

        let varying_expense: i64 = varying_buckets.values().copied().sum();
        let money_medium = income - fixed_expense;
        let varying_budget = money_medium - self.user_goal;
        let missed_goal_amount = if varying_expense > varying_budget {
            varying_expense - varying_budget
        } else {
            0
        };

        let mut tx = pool.begin().await?;

        let summary_id: i64 = sqlx::query_scalar!(r#"INSERT INTO monthly_summaries (month_year,
            income, fixed_expenses, varying_expenses,
            goal_amount, money_medium, miss_amount) VALUES (?,?,?,?,?,?,?)
            ON CONFLICT (month_year) DO UPDATE SET
            income = excluded.income,
            fixed_expenses = excluded.fixed_expenses,
            varying_expenses = excluded.varying_expenses,
            goal_amount = excluded.goal_amount,
            money_medium = excluded.money_medium,
            miss_amount = excluded.miss_amount
            RETURNING id"#,
            target_month,
            income,
            fixed_expense,
            varying_expense,
            self.user_goal,
            money_medium,
            missed_goal_amount
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM monthly_category_buckets WHERE summary_id = ?",
            summary_id
        )
        .execute(&mut *tx)
        .await?;

        for (category_name, bucket_amount) in &varying_buckets {
            if *bucket_amount > 0 {
                sqlx::query!(
                    "INSERT INTO monthly_category_buckets (summary_id, category_name, amount) VALUES (?, ?, ?)",
                    summary_id,
                    category_name,
                    bucket_amount
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }
}
