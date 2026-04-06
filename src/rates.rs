use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use numnum_core::config::config_dir;

const RATES_API: &str = "https://open.er-api.com/v6/latest/USD";
const DB_NAME: &str = "rates.db";

/// Hardcoded fallback rates (USD-relative, snapshot from API).
/// Used on first launch offline before any DB cache exists.
pub fn hardcoded_rates() -> HashMap<String, f64> {
    HashMap::from([
        ("AED", 3.6725), ("AFN", 64.0), ("ALL", 83.05), ("AMD", 377.09),
        ("ANG", 1.79), ("AOA", 919.34), ("ARS", 1387.72), ("AUD", 1.45),
        ("AWG", 1.79), ("AZN", 1.7), ("BAM", 1.7), ("BBD", 2.0),
        ("BDT", 122.69), ("BGN", 1.7), ("BHD", 0.376), ("BIF", 2980.13),
        ("BMD", 1.0), ("BND", 1.29), ("BOB", 6.9), ("BRL", 5.16),
        ("BSD", 1.0), ("BTC", 0.000012), ("BTN", 93.12), ("BWP", 13.88),
        ("BYN", 2.97), ("BZD", 2.0), ("CAD", 1.39), ("CDF", 2305.5),
        ("CHF", 0.8), ("CLF", 0.023), ("CLP", 918.68), ("CNH", 6.88),
        ("CNY", 6.89), ("COP", 3661.0), ("CRC", 464.84), ("CUP", 24.0),
        ("CVE", 95.74), ("CZK", 21.29), ("DJF", 177.72), ("DKK", 6.48),
        ("DOP", 60.58), ("DZD", 133.09), ("EGP", 54.45), ("ERN", 15.0),
        ("ETB", 155.48), ("ETH", 0.00055), ("EUR", 0.868), ("FJD", 2.25),
        ("FKP", 0.758), ("FOK", 6.48), ("GBP", 0.758), ("GEL", 2.7),
        ("GGP", 0.758), ("GHS", 11.03), ("GIP", 0.758), ("GMD", 74.2),
        ("GNF", 8771.02), ("GTQ", 7.65), ("GYD", 209.25), ("HKD", 7.84),
        ("HNL", 26.57), ("HRK", 6.54), ("HTG", 131.12), ("HUF", 333.93),
        ("IDR", 17003.11), ("ILS", 3.13), ("IMP", 0.758), ("INR", 93.13),
        ("IQD", 1311.68), ("IRR", 434525.31), ("ISK", 125.1), ("JEP", 0.758),
        ("JMD", 157.37), ("JOD", 0.709), ("JPY", 159.7), ("KES", 130.0),
        ("KGS", 87.47), ("KHR", 3987.73), ("KID", 1.45), ("KMF", 427.17),
        ("KRW", 1511.07), ("KWD", 0.308), ("KYD", 0.833), ("KZT", 471.05),
        ("LAK", 21937.51), ("LBP", 89500.0), ("LKR", 314.84), ("LRD", 183.23),
        ("LSL", 16.98), ("LYD", 6.39), ("MAD", 9.37), ("MDL", 17.54),
        ("MGA", 4194.47), ("MKD", 53.44), ("MMK", 2100.84), ("MNT", 3597.97),
        ("MOP", 8.07), ("MRU", 39.99), ("MUR", 46.92), ("MVR", 15.46),
        ("MWK", 1736.53), ("MXN", 17.89), ("MYR", 4.03), ("MZN", 63.63),
        ("NAD", 16.98), ("NGN", 1378.49), ("NIO", 36.78), ("NOK", 9.77),
        ("NPR", 149.0), ("NZD", 1.76), ("OMR", 0.384), ("PAB", 1.0),
        ("PEN", 3.45), ("PGK", 4.32), ("PHP", 60.37), ("PKR", 279.08),
        ("PLN", 3.71), ("PYG", 6435.8), ("QAR", 3.64), ("RON", 4.42),
        ("RSD", 101.76), ("RUB", 80.0), ("RWF", 1460.83), ("SAR", 3.75),
        ("SBD", 7.95), ("SCR", 14.52), ("SDG", 458.34), ("SEK", 9.48),
        ("SGD", 1.29), ("SHP", 0.758), ("SLE", 24.66), ("SLL", 24661.73),
        ("SOS", 571.62), ("SRD", 37.55), ("SSP", 4587.26), ("STN", 21.27),
        ("SYP", 112.4), ("SZL", 16.98), ("THB", 32.66), ("TJS", 9.56),
        ("TMT", 3.5), ("TND", 2.92), ("TOP", 2.38), ("TRY", 44.58),
        ("TTD", 6.78), ("TVD", 1.45), ("TWD", 32.04), ("TZS", 2600.21),
        ("UAH", 43.69), ("UGX", 3745.41), ("USD", 1.0), ("UYU", 40.46),
        ("UZS", 12247.4), ("VES", 474.06), ("VND", 26237.91), ("VUV", 119.45),
        ("WST", 2.73), ("XAF", 569.56), ("XCD", 2.7), ("XCG", 1.79),
        ("XDR", 0.735), ("XOF", 569.56), ("XPF", 103.61), ("YER", 238.61),
        ("ZAR", 16.98), ("ZMW", 19.29), ("ZWG", 25.37), ("ZWL", 25.37),
    ].map(|(k, v)| (k.to_string(), v)))
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
