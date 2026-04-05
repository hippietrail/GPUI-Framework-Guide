use crate::types::*;

pub fn format_value(value: &Value, unit_table: &UnitTable, currency_table: &CurrencyTable) -> String {
    match value {
        Value::Number(n) => format_number(*n),
        Value::NumberRepr(n, repr) => format_number_repr(*n, *repr),
        Value::WithUnit(n, id) => {
            match unit_table.get(*id) {
                Some(unit) => format!("{} {}", format_number(*n), unit.display),
                None => format_number(*n),
            }
        }
        Value::WithCurrency(n, id) => {
            match currency_table.get(*id) {
                Some(currency) => format_currency(*n, &currency.display_format),
                None => format_number(*n),
            }
        }
        Value::Percent(n) => format!("{} %", format_number(*n * 100.0)),
        Value::None => String::new(),
    }
}

pub fn format_number(n: f64) -> String {
    if n.is_nan() { return "NaN".to_string(); }
    if n.is_infinite() { return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string(); }

    // Normalize -0.0 to 0.0, and round near-zero values (floating point drift) to exactly 0.0
    let n = if n == 0.0 || n.abs() < 1e-10 { 0.0 } else { n };

    // Exact integers within safe i64 range
    if n == n.floor() && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }

    // Very large or very small numbers: use scientific notation
    if n != 0.0 && (n.abs() >= 1e15 || n.abs() < 0.01) {
        let s = format!("{:.6e}", n);
        // Trim trailing zeros in the mantissa: "1.200000e3" -> "1.2e3"
        if let Some(e_pos) = s.find('e') {
            let mantissa = s[..e_pos].trim_end_matches('0').trim_end_matches('.');
            let exponent = &s[e_pos..];
            return format!("{}{}", mantissa, exponent);
        }
        return s;
    }

    // Normal decimals: show up to 2 decimal places (matching smaller-unit preference)
    let s = format!("{:.2}", n);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

pub fn format_number_repr(n: f64, repr: NumRepr) -> String {
    let i = n as i64;
    match repr {
        NumRepr::Hex => format!("0x{:x}", i),
        NumRepr::Binary => format!("0b{:b}", i),
        NumRepr::Octal => format!("0o{:o}", i),
        NumRepr::Decimal => format_number(n),
    }
}

fn format_currency(n: f64, format: &str) -> String {
    let num_str = format_number(n);
    if format.contains("%@") {
        format.replace("%@", &num_str)
    } else {
        // Fallback: just show number + code
        num_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_integer() {
        assert_eq!(format_number(42.0), "42");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-5.0), "-5");
    }

    #[test]
    fn test_format_decimal() {
        assert_eq!(format_number(3.14), "3.14");
        assert_eq!(format_number(0.5), "0.5");
        assert_eq!(format_number(1.10), "1.1");
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency(10.0, "$%@"), "$10");
        assert_eq!(format_currency(86.83, "\u{20AC} %@"), "\u{20AC} 86.83");
        assert_eq!(format_currency(150.0, "%@ SFr."), "150 SFr.");
    }
}
