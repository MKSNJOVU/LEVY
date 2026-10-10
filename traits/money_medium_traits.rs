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
}
