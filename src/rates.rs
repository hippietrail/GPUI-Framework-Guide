use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use numnum_core::config::config_dir;

const RATES_API: &str = "https://open.er-api.com/v6/latest/USD";
const DB_NAME: &str = "rates.db";

/// Hardcoded fallback rates (USD-relative) used when no database exists
/// and the network fetch fails (e.g. first launch while offline).
pub fn hardcoded_rates() -> HashMap<String, f64> {
    let mut m = HashMap::new();
    m.insert("USD".into(), 1.0);
    m.insert("EUR".into(), 0.87);
    m.insert("GBP".into(), 0.76);
    m.insert("JPY".into(), 149.5);
    m.insert("CHF".into(), 0.88);
    m.insert("CAD".into(), 1.36);
    m.insert("AUD".into(), 1.53);
    m.insert("CNY".into(), 7.24);
    m.insert("INR".into(), 83.5);
    m.insert("KRW".into(), 1330.0);
    m.insert("RUB".into(), 92.0);
    m.insert("BRL".into(), 5.0);
    m.insert("MXN".into(), 17.2);
    m.insert("SEK".into(), 10.5);
    m.insert("NOK".into(), 10.6);
    m.insert("DKK".into(), 6.9);
    m.insert("PLN".into(), 4.0);
    m.insert("CZK".into(), 23.0);
    m.insert("HUF".into(), 360.0);
    m.insert("TRY".into(), 27.0);
    m.insert("SGD".into(), 1.35);
    m.insert("HKD".into(), 7.82);
    m.insert("THB".into(), 35.5);
    m.insert("ZAR".into(), 18.5);
    m.insert("AED".into(), 3.67);
    m.insert("SAR".into(), 3.75);
    m.insert("ILS".into(), 3.7);
    m.insert("UAH".into(), 37.5);
    m.insert("NZD".into(), 1.63);
    m.insert("BTC".into(), 0.000015);
    m.insert("ETH".into(), 0.00049);
    m
}

pub struct RateCache {
    db_path: PathBuf,
}

impl RateCache {
    pub fn new() -> Self {
        RateCache {
            db_path: config_dir().join(DB_NAME),
        }
    }

    /// Ensure the config directory and SQLite database/table exist.
    async fn ensure_db(&self, pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS rates (\
                code TEXT PRIMARY KEY, \
                rate_to_usd REAL NOT NULL, \
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))\
            )",
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Load all cached rates from the SQLite database.
    async fn load_cached_rates(
        &self,
        pool: &sqlx::SqlitePool,
    ) -> Result<HashMap<String, f64>, sqlx::Error> {
        let rows: Vec<(String, f64)> =
            sqlx::query_as("SELECT code, rate_to_usd FROM rates")
                .fetch_all(pool)
                .await?;
        let map: HashMap<String, f64> = rows.into_iter().collect();
        Ok(map)
    }

    /// Store rates into the SQLite database (upsert).
    async fn store_rates(
        &self,
        pool: &sqlx::SqlitePool,
        rates: &HashMap<String, f64>,
    ) -> Result<(), sqlx::Error> {
        for (code, rate) in rates {
            sqlx::query(
                "INSERT INTO rates (code, rate_to_usd, updated_at) \
                 VALUES (?, ?, datetime('now')) \
                 ON CONFLICT(code) DO UPDATE SET rate_to_usd = excluded.rate_to_usd, \
                 updated_at = excluded.updated_at",
            )
            .bind(code)
            .bind(rate)
            .execute(pool)
            .await?;
        }
        Ok(())
    }

    /// Fetch live rates from the API. Returns the rates map on success.
    fn fetch_live_rates(&self) -> Result<HashMap<String, f64>, Box<dyn std::error::Error>> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(3)))
            .build();
        let agent = ureq::Agent::new_with_config(config);
        let body: String = agent.get(RATES_API).call()?.body_mut().read_to_string()?;
        let json: serde_json::Value = serde_json::from_str(&body)?;

        let result = json
            .get("result")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if result != "success" {
            return Err(format!("API returned result={}", result).into());
        }

        let rates_obj = json
            .get("rates")
            .and_then(|v| v.as_object())
            .ok_or("missing 'rates' object in API response")?;

        let mut map = HashMap::new();
        for (code, val) in rates_obj {
            if let Some(rate) = val.as_f64() {
                map.insert(code.clone(), rate);
            }
        }
        Ok(map)
    }

    /// Fast startup: load cached rates from SQLite, or fall back to hardcoded.
    /// No network calls. Returns instantly.
    pub fn get_cached_rates(&self) -> HashMap<String, f64> {
        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let db_url = format!("sqlite:{}?mode=rwc", self.db_path.display());

        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return hardcoded_rates(),
        };

        rt.block_on(async {
            let pool = match sqlx::SqlitePool::connect(&db_url).await {
                Ok(p) => p,
                Err(_) => return hardcoded_rates(),
            };
            if self.ensure_db(&pool).await.is_err() {
                return hardcoded_rates();
            }
            let cached = self.load_cached_rates(&pool).await.unwrap_or_default();
            pool.close().await;
            if cached.is_empty() { hardcoded_rates() } else { cached }
        })
    }

    /// Fetch live rates from the network and store in SQLite.
    /// Designed to run in a background thread. Returns the new rates on success.
    pub fn fetch_and_store(&self) -> Option<HashMap<String, f64>> {
        let live = self.fetch_live_rates().ok()?;

        if let Some(parent) = self.db_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let db_url = format!("sqlite:{}?mode=rwc", self.db_path.display());

        if let Ok(rt) = tokio::runtime::Runtime::new() {
            rt.block_on(async {
                if let Ok(pool) = sqlx::SqlitePool::connect(&db_url).await {
                    let _ = self.ensure_db(&pool).await;
                    let _ = self.store_rates(&pool, &live).await;
                    pool.close().await;
                }
            });
        }
        Some(live)
    }
}

/// Convenience: load rates and apply them to a CurrencyTable.
pub fn apply_rates(table: &mut numnum_core::types::CurrencyTable, rates: &HashMap<String, f64>) {
    for (code, rate) in rates {
        table.update_rate(code, *rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardcoded_rates_sanity() {
        let rates = hardcoded_rates();
        assert_eq!(rates.get("USD"), Some(&1.0));
        assert!(rates.get("EUR").is_some());
        assert!(rates.get("INR").is_some());
        assert!(rates.len() >= 30);
    }

    #[test]
    fn test_apply_rates_updates_currency_table() {
        let mut table = numnum_core::types::CurrencyTable::new();
        let old_inr = table.lookup("INR")
            .and_then(|id| table.get(id))
            .map(|d| d.rate_to_usd)
            .unwrap();
        assert!((old_inr - 83.5).abs() < 0.01); // hardcoded default

        let mut rates = HashMap::new();
        rates.insert("INR".into(), 85.0);
        apply_rates(&mut table, &rates);

        let new_inr = table.lookup("INR")
            .and_then(|id| table.get(id))
            .map(|d| d.rate_to_usd)
            .unwrap();
        assert!((new_inr - 85.0).abs() < 0.001);
    }

    #[test]
    fn test_rate_cache_get_rates_returns_nonempty() {
        let cache = RateCache::new();
        let rates = cache.get_cached_rates();
        // Should always return something (live, cached, or hardcoded)
        assert!(!rates.is_empty());
        assert!(rates.contains_key("USD"));
        assert!(rates.contains_key("EUR"));
    }

    #[test]
    fn test_fetch_live_rates() {
        let cache = RateCache::new();
        // This test requires network access; skip gracefully if offline
        match cache.fetch_live_rates() {
            Ok(rates) => {
                assert!(rates.len() > 100, "API should return 160+ currencies");
                assert_eq!(rates.get("USD"), Some(&1.0));
                // INR should be a plausible number (50-120 range)
                let inr = rates.get("INR").expect("INR should exist");
                assert!(*inr > 50.0 && *inr < 120.0, "INR rate {} looks implausible", inr);
            }
            Err(e) => {
                eprintln!("Skipping live rate test (network unavailable): {}", e);
            }
        }
    }
}
