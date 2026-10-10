use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tokio::*;
use crate::money_medium_structs::*;
pub trait ILevyOrchestration: IUserSavingsGoal + ICalculateMoneyMedium {
}

pub trait IUserSavingsGoal {
async fn log_user_savings_goal(&self, pool: &SqlitePool) -> Result<i64, sqlx::Error>;
}

pub trait ICalculateMoneyMedium{
 async fn user_income(&self) -> i64;
 async fn user_expense(&self) -> i64;
 async fn disposable_income(&self) -> i64{
  let income = self.user_income().await;
  let expenses = self.user_expense().await;
  income - expenses
 }
}

// Implementations
impl ICalculateMoneyMedium for UserMoneyMedium{
 async fn user_income(&self) -> i64 {
  self.user_income.monthly_income
 }
 async fn user_expense(&self) -> i64 {
 let total_fixed_costs: i64 = self.user_expenses.fixed_expenses
     .iter()
     .map(|category| category.amount.unwrap_or(0))
     .sum();

  total_fixed_costs
 }
}

impl IUserSavingsGoal for UserMoneyMedium{
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

// impl ILevyOrchestration for UserMoneyMedium{}
