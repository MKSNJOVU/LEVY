
#[derive(Debug, Clone)]
pub struct UserMoneyMedium{
    pub user_goal: i64,
    pub user_income: UserIncome,
    pub user_expenses: UserExpenses,
    pub user_money_medium: i64,
}
#[derive(Debug, Clone)]
pub struct UserIncome{
    pub monthly_income: i64,
    pub current_earnings: Vec<Option<i64>>,
}
#[derive(Debug, Clone)]
pub struct UserExpenses{
    pub fixed_expenses: Vec<ExpenseCategory>,
    pub varying_expenses: Vec<ExpenseCategory>,
}
#[derive(Debug, Clone)]
pub struct ExpenseCategory{
    pub name: String,
    pub description: Option<String>,
    pub amount: Option<i64>,
    pub is_active: bool,
}

