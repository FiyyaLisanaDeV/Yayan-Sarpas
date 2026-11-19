//! Validasi nilai config per tipe (validation_json dari §7).

use chrono::NaiveDate;
use serde_json::Value;

use crate::error::{AppError, AppResult};

/// Parse string nilai menjadi Value sesuai tipe config.
pub fn parse_nilai(tipe: &str, raw: &str) -> Result<Value, String> {
    match tipe {
        "boolean" => raw
            .parse::<bool>()
            .map(Value::Bool)
            .map_err(|_| format!("'{raw}' bukan boolean (true/false)")),
        "number" => raw
            .parse::<i64>()
            .map(Value::from)
            .map_err(|_| format!("'{raw}' bukan angka bulat")),
        "decimal" => raw
            .parse::<f64>()
            .map(|v| serde_json::json!(v))
            .map_err(|_| format!("'{raw}' bukan angka desimal")),
        "json" => serde_json::from_str(raw).map_err(|e| format!("json tidak valid: {e}")),
        _ => Ok(Value::String(raw.to_string())),
    }
}

/// Validasi nilai baru sebelum disimpan (sesuai tipe + validation_json).
pub fn validate_value(tipe: &str, validation: &Option<Value>, raw: &str) -> AppResult<()> {
    let bad = |msg: String| AppError::Validation(msg);

    match tipe {
        "number" => {
            let v: i64 = raw
                .parse()
                .map_err(|_| bad(format!("'{raw}' bukan angka bulat")))?;
            check_minmax_i64(validation, v).map_err(bad)?;
        }
        "decimal" => {
            let v: f64 = raw
                .parse::<f64>()
                .map_err(|_| bad(format!("'{raw}' bukan angka desimal")))?;
            if let Some(min) = validation
                .as_ref()
                .and_then(|j| j.get("min"))
                .and_then(|m| m.as_f64())
            {
                if v < min {
                    return Err(bad(format!("nilai minimal adalah {min}")));
                }
            }
            if let Some(max) = validation
                .as_ref()
                .and_then(|j| j.get("max"))
                .and_then(|m| m.as_f64())
            {
                if v > max {
                    return Err(bad(format!("nilai maksimal adalah {max}")));
                }
            }
        }
        "boolean" => {
            raw.parse::<bool>()
                .map_err(|_| bad(format!("'{raw}' bukan boolean (true/false)")))?;
        }
        "enum" => {
            let options: Vec<String> = validation
                .as_ref()
                .and_then(|j| j.get("options"))
                .and_then(|o| o.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            if !options.is_empty() && !options.iter().any(|o| o == raw) {
                return Err(bad(format!(
                    "'{raw}' bukan pilihan valid. Pilihan: {}",
                    options.join(", ")
                )));
            }
        }
        "json" => {
            serde_json::from_str::<Value>(raw)
                .map_err(|e| bad(format!("json tidak valid: {e}")))?;
        }
        "date" => {
            NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .map_err(|_| bad(format!("format tanggal harus YYYY-MM-DD, bukan '{raw}'")))?;
        }
        "duration_days" => {
            let v: i64 = raw
                .parse()
                .map_err(|_| bad(format!("'{raw}' bukan angka")))?;
            if v < 0 {
                return Err(bad("durasi tidak boleh negatif".to_string()));
            }
        }
        "color" => {
            let ok = raw.len() == 7
                && raw.starts_with('#')
                && raw[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !ok {
                return Err(bad(format!(
                    "format warna harus hex #RRGGBB, bukan '{raw}'"
                )));
            }
        }
        "email" => {
            let ok = raw.contains('@') && raw.contains('.') && raw.len() >= 5;
            if !ok {
                return Err(bad(format!("'{raw}' bukan email valid")));
            }
        }
        "url" => {
            if !(raw.starts_with("http://") || raw.starts_with("https://")) {
                return Err(bad(format!("'{raw}' bukan URL valid (harus http/https)")));
            }
        }
        "text" | "template" => {
            let max = validation
                .as_ref()
                .and_then(|j| j.get("max_length"))
                .and_then(|m| m.as_u64())
                .unwrap_or(u64::MAX);
            if raw.len() as u64 > max {
                return Err(bad(format!("panjang maksimal {max} karakter")));
            }
        }
        other => return Err(bad(format!("tipe config tidak dikenal: {other}"))),
    }
    Ok(())
}

fn check_minmax_i64(validation: &Option<Value>, v: i64) -> Result<(), String> {
    if let Some(min) = validation
        .as_ref()
        .and_then(|j| j.get("min"))
        .and_then(|m| m.as_i64())
    {
        if v < min {
            return Err(format!("nilai minimal adalah {min}"));
        }
    }
    if let Some(max) = validation
        .as_ref()
        .and_then(|j| j.get("max"))
        .and_then(|m| m.as_i64())
    {
        if v > max {
            return Err(format!("nilai maksimal adalah {max}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(json: &str) -> Option<Value> {
        Some(serde_json::from_str(json).unwrap())
    }

    #[test]
    fn number_minmax() {
        assert!(validate_value("number", &v(r#"{"min":0,"max":999999}"#), "5").is_ok());
        assert!(validate_value("number", &v(r#"{"min":0,"max":999999}"#), "-1").is_err());
        assert!(validate_value("number", &v(r#"{"min":0,"max":999999}"#), "1000000").is_err());
        assert!(validate_value("number", &v(r#"{"min":0,"max":999999}"#), "abc").is_err());
    }

    #[test]
    fn decimal() {
        assert!(validate_value("decimal", &v(r#"{"min":0}"#), "11.00").is_ok());
        assert!(validate_value("decimal", &v(r#"{"min":0}"#), "-5").is_err());
    }

    #[test]
    fn enum_harus_sesuai_options() {
        assert!(validate_value("enum", &v(r#"{"options":["KERJA","KALENDER"]}"#), "KERJA").is_ok());
        assert!(
            validate_value("enum", &v(r#"{"options":["KERJA","KALENDER"]}"#), "MINGGU").is_err()
        );
    }

    #[test]
    fn boolean_dan_json() {
        assert!(validate_value("boolean", &None, "true").is_ok());
        assert!(validate_value("boolean", &None, "iya").is_err());
        assert!(validate_value("json", &None, r#"{"a":1}"#).is_ok());
        assert!(validate_value("json", &None, "bukan json").is_err());
    }

    #[test]
    fn color_dan_date() {
        assert!(validate_value("color", &v(r#"{"format":"hex"}"#), "#1E88E5").is_ok());
        assert!(validate_value("color", &v(r#"{"format":"hex"}"#), "biru").is_err());
        assert!(validate_value("date", &None, "2026-11-15").is_ok());
        assert!(validate_value("date", &None, "15/11/2026").is_err());
    }
}
