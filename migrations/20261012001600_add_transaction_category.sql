-- Plaid personal_finance_category.primary, for example FOOD_AND_DRINK or RENT_AND_UTILITIES.
ALTER TABLE transactions ADD COLUMN category TEXT;
