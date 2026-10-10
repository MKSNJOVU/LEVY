use serde::Deserialize;
use serde_json::{Value, json};

pub struct PlaidClient {
    http: reqwest::Client,
    base_url: String,
    environment: String,
    client_id: String,
    secret: String,
    version: String,
}

pub struct PlaidConfig {
    pub base_url: String,
    pub environment: String,
    pub client_id: String,
    pub secret: String,
    pub version: String,
}

pub struct ItemCredentials {
    pub access_token: String,
    pub item_id: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncPage {
    #[serde(default)]
    pub added: Vec<PlaidTransaction>,
    #[serde(default)]
    pub modified: Vec<PlaidTransaction>,
    #[serde(default)]
    pub removed: Vec<RemovedTransaction>,
    pub next_cursor: String,
    pub has_more: bool,
    #[serde(default)]
    pub accounts: Vec<PlaidAccount>,
}

#[derive(Debug, Deserialize)]
pub struct PlaidTransaction {
    pub transaction_id: String,
    pub account_id: String,
    pub amount: serde_json::Number,
    pub date: String,
    pub name: Option<String>,
    #[serde(default)]
    pub pending: bool,
    pub personal_finance_category: Option<PersonalFinanceCategory>,
}
#[derive(Debug, Deserialize)]
pub struct PersonalFinanceCategory{
    pub primary: String,
    pub detailed: String,
}

#[derive(Debug, Deserialize)]
pub struct RemovedTransaction {
    pub transaction_id: String,
}

#[derive(Debug, Deserialize)]
pub struct PlaidAccount {
    pub account_id: String,
    pub name: String,
    pub balances: PlaidBalances,
}

#[derive(Debug, Deserialize)]
pub struct PlaidBalances {
    pub current: Option<serde_json::Number>,
}

impl PlaidClient {
    pub fn from_env() -> Result<Self, String> {
        let environment =
            std::env::var("PLAID_ENV").map_err(|_| "PLAID_ENV is not set".to_string())?;
        let base_url = match environment.as_str() {
            "sandbox" => "https://sandbox.plaid.com",
            "development" => "https://development.plaid.com",
            "production" => "https://production.plaid.com",
            other => {
                return Err(format!(
                    "PLAID_ENV must be sandbox, development, or production (got {other})"
                ));
            }
        };
        Self::new(PlaidConfig {
            base_url: base_url.to_string(),
            environment,
            client_id: std::env::var("PLAID_CLIENT_ID")
                .map_err(|_| "PLAID_CLIENT_ID is not set".to_string())?,
            secret: std::env::var("PLAID_SECRET")
                .map_err(|_| "PLAID_SECRET is not set".to_string())?,
            version: std::env::var("PLAID_VERSION")
                .map_err(|_| "PLAID_VERSION is not set".to_string())?,
        })
    }

    pub fn new(config: PlaidConfig) -> Result<Self, String> {
        install_crypto_provider();
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|err| err.to_string())?;
        Ok(Self {
            http,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            environment: config.environment,
            client_id: config.client_id,
            secret: config.secret,
            version: config.version,
        })
    }

    pub fn environment(&self) -> &str {
        &self.environment
    }

    pub async fn create_link_token(&self) -> Result<String, String> {
        let value = self
            .post(
                "/link/token/create",
                json!({
                    "client_name": "Levy",
                    "language": "en",
                    "country_codes": ["US"],
                    "user": { "client_user_id": "levy-local" },
                    "products": ["transactions"]
                }),
            )
            .await?;
        required_string(&value, "link_token")
    }

    pub async fn exchange_public_token(
        &self,
        public_token: &str,
    ) -> Result<ItemCredentials, String> {
        let value = self
            .post(
                "/item/public_token/exchange",
                json!({ "public_token": public_token }),
            )
            .await?;
        Ok(ItemCredentials {
            access_token: required_string(&value, "access_token")?,
            item_id: required_string(&value, "item_id")?,
        })
    }

    #[allow(dead_code)]
    pub async fn create_sandbox_public_token(&self) -> Result<String, String> {
        let value = self
            .post(
                "/sandbox/public_token/create",
                json!({
                    "institution_id": "ins_109508",
                    "initial_products": ["transactions"]
                }),
            )
            .await?;
        required_string(&value, "public_token")
    }

    pub async fn transactions_sync(
        &self,
        access_token: &str,
        cursor: Option<&str>,
    ) -> Result<SyncPage, String> {
        let mut body = json!({
            "access_token": access_token,
            "count": 100
        });
        if let Some(cursor) = cursor.filter(|cursor| !cursor.is_empty()) {
            body["cursor"] = json!(cursor);
        }
        let value = self.post("/transactions/sync", body).await?;
        serde_json::from_value(value)
            .map_err(|err| format!("invalid transactions/sync response: {err}"))
    }

    async fn post(&self, path: &str, mut body: Value) -> Result<Value, String> {
        if let Some(object) = body.as_object_mut() {
            object.insert("client_id".to_string(), json!(self.client_id));
            object.insert("secret".to_string(), json!(self.secret));
        }
        let response = self
            .http
            .post(format!("{}{path}", self.base_url))
            .header("PLAID-CLIENT-ID", &self.client_id)
            .header("PLAID-SECRET", &self.secret)
            .header("Plaid-Version", &self.version)
            .json(&body)
            .send()
            .await
            .map_err(|err| format!("Plaid {path} request failed: {err}"))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|err| format!("Plaid {path} response could not be read: {err}"))?;
        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        if !status.is_success() {
            return Err(plaid_error(path, status, &value));
        }
        if value.is_null() {
            return Err(format!("Plaid {path} returned a non-JSON response"));
        }
        Ok(value)
    }
}

fn install_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Plaid response missing {field}"))
}

fn plaid_error(path: &str, status: reqwest::StatusCode, value: &Value) -> String {
    let code = value
        .get("error_code")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let message = value
        .get("error_message")
        .and_then(Value::as_str)
        .unwrap_or("request failed");
    format!("Plaid {path} failed ({status}): {code}: {message}")
}

/// Convert a Plaid dollar amount to integer cents, rounding half away from zero.
pub fn dollars_to_cents(amount: &str) -> Result<i64, String> {
    let amount = amount.trim();
    if amount.is_empty() {
        return Err("amount is empty".to_string());
    }
    let (negative, unsigned) = if let Some(rest) = amount.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = amount.strip_prefix('+') {
        (false, rest)
    } else {
        (false, amount)
    };
    if unsigned.is_empty() || unsigned.contains(['e', 'E', '+', '-']) {
        return Err(format!("unsupported amount {amount}"));
    }
    let (whole, frac) = match unsigned.split_once('.') {
        Some((whole, frac)) => (whole, frac),
        None => (unsigned, ""),
    };
    if whole.is_empty() && frac.is_empty()
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !frac.chars().all(|c| c.is_ascii_digit())
    {
        return Err(format!("invalid amount {amount}"));
    }
    let dollars: i64 = if whole.is_empty() {
        0
    } else {
        whole
            .parse()
            .map_err(|_| format!("invalid amount {amount}"))?
    };
    let mut digits: Vec<char> = frac.chars().collect();
    while digits.len() < 3 {
        digits.push('0');
    }
    let cents_two: i64 = format!("{}{}", digits[0], digits[1])
        .parse()
        .map_err(|_| format!("invalid amount {amount}"))?;
    let round_up = digits[2].to_digit(10).unwrap_or(0) >= 5;
    let mut cents = dollars
        .checked_mul(100)
        .and_then(|value| value.checked_add(cents_two))
        .ok_or_else(|| format!("amount overflow {amount}"))?;
    if round_up {
        cents = cents
            .checked_add(1)
            .ok_or_else(|| format!("amount overflow {amount}"))?;
    }
    if negative {
        cents = cents
            .checked_neg()
            .ok_or_else(|| format!("amount overflow {amount}"))?;
    }
    Ok(cents)
}

#[cfg(test)]
mod tests {
    use super::dollars_to_cents;

    #[test]
    fn rounds_half_away_from_zero() {
        assert_eq!(dollars_to_cents("1.225").unwrap(), 123);
        assert_eq!(dollars_to_cents("-1.225").unwrap(), -123);
        assert_eq!(dollars_to_cents("1.224").unwrap(), 122);
        assert_eq!(dollars_to_cents("-1.224").unwrap(), -122);
        assert_eq!(dollars_to_cents("0.005").unwrap(), 1);
        assert_eq!(dollars_to_cents("-0.005").unwrap(), -1);
        assert_eq!(dollars_to_cents("0.004").unwrap(), 0);
        assert_eq!(dollars_to_cents("12.34").unwrap(), 1234);
        assert_eq!(dollars_to_cents("-5.005").unwrap(), -501);
        assert_eq!(dollars_to_cents("110.5").unwrap(), 11050);
        assert_eq!(dollars_to_cents("10").unwrap(), 1000);
        assert_eq!(dollars_to_cents("+15.00").unwrap(), 1500);
    }
}
