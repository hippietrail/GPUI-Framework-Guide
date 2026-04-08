use crate::types::*;

/// Full precision format for clipboard copy. Shows all significant digits.
pub fn format_value_full_precision(value: &Value, unit_table: &UnitTable, currency_table: &CurrencyTable, fmt: NumberFormat) -> String {
    let format_full = |n: f64| -> String {
        if n.is_nan() || n.is_infinite() { return format_number(n, 2, fmt); }
        if n == 0.0 { return "0".to_string(); }
        if n == n.floor() && n.abs() < 1e15 {
            return format_with_separators(&format!("{}", n as i64), fmt);
        }
        let s = format!("{}", n);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        format_with_separators(s, fmt)
    };
    match value {
        Value::Number(n) => format_full(*n),
        Value::NumberRepr(n, repr) => format_number_repr(*n, *repr),
        Value::WithUnit(n, id) => {
            match unit_table.get(*id) {
                Some(unit) => format!("{} {}", format_full(*n), unit.display),
                None => format_full(*n),
            }
        }
        Value::WithCompoundUnit(n, factors) => {
            format!("{} {}", format_full(*n), format_compound_unit(factors, unit_table))
        }
        Value::WithCurrency(n, id) => {
            match currency_table.get(*id) {
                Some(currency) => format_currency_with_precision(*n, &currency.display_format, None, fmt).replace(
                    &format_number(*n, 2, fmt), &format_full(*n)
                ),
                None => format_full(*n),
            }
        }
        Value::Percent(n) => format!("{} %", format_full(*n * 100.0)),
        Value::None => String::new(),
    }
}

pub fn format_value(value: &Value, unit_table: &UnitTable, currency_table: &CurrencyTable) -> String {
    format_value_with_precision(value, unit_table, currency_table, 2, NumberFormat::US)
}

pub fn format_value_with_precision(value: &Value, unit_table: &UnitTable, currency_table: &CurrencyTable, precision: u32, fmt: NumberFormat) -> String {
    match value {
        Value::Number(n) => format_number(*n, precision, fmt),
        Value::NumberRepr(n, repr) => format_number_repr(*n, *repr),
        Value::WithUnit(n, id) => {
            match unit_table.get(*id) {
                Some(unit) => format!("{} {}", format_number(*n, precision, fmt), unit.display),
                None => format_number(*n, precision, fmt),
            }
        }
        Value::WithCompoundUnit(n, factors) => {
            format!("{} {}", format_number(*n, precision, fmt), format_compound_unit(factors, unit_table))
        }
        Value::WithCurrency(n, id) => {
            match currency_table.get(*id) {
                Some(currency) => format_currency_with_precision(*n, &currency.display_format, Some(precision), fmt),
                None => format_number(*n, precision, fmt),
            }
        }
        Value::Percent(n) => format!("{} %", format_number(*n * 100.0, precision, fmt)),
        Value::None => String::new(),
    }
}

pub fn format_number(n: f64, precision: u32, fmt: NumberFormat) -> String {
    if n.is_nan() { return "NaN".to_string(); }
    if n.is_infinite() { return if n > 0.0 { "Infinity" } else { "-Infinity" }.to_string(); }

    // Normalize -0.0 to 0.0, and round near-zero values (floating point drift) to exactly 0.0
    let n = if n == 0.0 || n.abs() < 1e-10 { 0.0 } else { n };

    // Exact integers within safe i64 range
    if n == n.floor() && n.abs() < 1e15 {
        return format_with_separators(&format!("{}", n as i64), fmt);
    }

    // Very large or very small numbers: use scientific notation
    if n != 0.0 && (n.abs() >= 1e15 || n.abs() < 0.01) {
        let s = format!("{:.6e}", n);
        if let Some(e_pos) = s.find('e') {
            let mantissa = s[..e_pos].trim_end_matches('0').trim_end_matches('.');
            let exponent = &s[e_pos..];
            return format!("{}{}", mantissa, exponent);
        }
        return s;
    }

    // Normal decimals: show up to `precision` decimal places
    let s = format!("{:.*}", precision as usize, n);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format_with_separators(s, fmt)
}

/// Insert locale-appropriate separators into a number string.
/// Input always uses '.' as decimal separator (from Rust's format!).
fn format_with_separators(s: &str, fmt: NumberFormat) -> String {
    let (sign, rest) = if s.starts_with('-') { ("-", &s[1..]) } else { ("", s) };
    let (int_part, frac_part) = match rest.find('.') {
        Some(dot) => (&rest[..dot], &rest[dot..]), // frac_part includes the '.'
        None => (rest, ""),
    };

    if int_part.len() <= 3 {
        let result = format!("{}{}{}", sign, int_part, frac_part);
        if matches!(fmt, NumberFormat::European) {
            return result.replace('.', ",");
        }
        return result;
    }

    let (thousands_sep, decimal_sep) = match fmt {
        NumberFormat::US | NumberFormat::Indian => (',', '.'),
        NumberFormat::European => ('.', ','),
    };

    let mut result = String::with_capacity(s.len() + int_part.len() / 2);
    result.push_str(sign);

    match fmt {
        NumberFormat::Indian => {
            // Indian: last 3 digits as first group, then groups of 2
            let last3_start = int_part.len() - 3;
            let prefix = &int_part[..last3_start];
            // Groups of 2 from right in the prefix
            let first_group = prefix.len() % 2;
            if first_group > 0 {
                result.push_str(&prefix[..first_group]);
            }
            for (i, chunk) in prefix[first_group..].as_bytes().chunks(2).enumerate() {
                if i > 0 || first_group > 0 {
                    result.push(thousands_sep);
                }
                result.push_str(std::str::from_utf8(chunk).unwrap());
            }
            if !prefix.is_empty() {
                result.push(thousands_sep);
            }
            result.push_str(&int_part[last3_start..]);
        }
        NumberFormat::US | NumberFormat::European => {
            // Groups of 3 from right
            let first_group = int_part.len() % 3;
            if first_group > 0 {
                result.push_str(&int_part[..first_group]);
            }
            for (i, chunk) in int_part[first_group..].as_bytes().chunks(3).enumerate() {
                if i > 0 || first_group > 0 {
                    result.push(thousands_sep);
                }
                result.push_str(std::str::from_utf8(chunk).unwrap());
            }
        }
    }

    // Append fractional part with correct decimal separator
    if !frac_part.is_empty() {
        result.push(decimal_sep);
        result.push_str(&frac_part[1..]); // skip the '.' from Rust's format
    }
    result
}

pub fn format_number_repr(n: f64, repr: NumRepr) -> String {
    let i = n as i64;
    match repr {
        NumRepr::Hex => format!("0x{:x}", i),
        NumRepr::Binary => format!("0b{:b}", i),
        NumRepr::Octal => format!("0o{:o}", i),
        NumRepr::Decimal => format_number(n, 2, NumberFormat::US),
        NumRepr::Scientific => {
            if n == 0.0 { return "0e0".to_string(); }
            let exp = n.abs().log10().floor() as i32;
            let mantissa = n / 10f64.powi(exp);
            // Format mantissa, trimming trailing zeros
            let s = format!("{:.6}", mantissa);
            let s = s.trim_end_matches('0').trim_end_matches('.');
            format!("{}e{}", s, exp)
        }
    }
}

fn format_compound_unit(factors: &[(UnitId, i8)], unit_table: &UnitTable) -> String {
    let mut numerator = Vec::new();
    let mut denominator = Vec::new();

    for &(id, exp) in factors {
        let display = unit_table.get(id)
            .map(|u| u.display.as_str())
            .unwrap_or("?");
        if exp > 0 {
            numerator.push((display, exp));
        } else if exp < 0 {
            denominator.push((display, -exp));
        }
    }

    let format_part = |parts: &[(&str, i8)]| -> String {
        parts.iter().map(|(name, exp)| {
            match exp {
                1 => name.to_string(),
                2 => format!("{}\u{00B2}", name),   // ²
                3 => format!("{}\u{00B3}", name),   // ³
                _ => format!("{}^{}", name, exp),
            }
        }).collect::<Vec<_>>().join("\u{00B7}") // middle dot for multiplication
    };

    if denominator.is_empty() {
        format_part(&numerator)
    } else if numerator.is_empty() {
        format!("1/{}", format_part(&denominator))
    } else {
        format!("{}/{}", format_part(&numerator), format_part(&denominator))
    }
}

fn format_currency_with_precision(n: f64, fmt_str: &str, precision: Option<u32>, fmt: NumberFormat) -> String {
    let num_str = format_number(n, precision.unwrap_or(2), fmt);
    if fmt_str.contains("%@") {
        fmt_str.replace("%@", &num_str)
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
        assert_eq!(format_number(42.0, 2, NumberFormat::US), "42");
        assert_eq!(format_number(0.0, 2, NumberFormat::US), "0");
        assert_eq!(format_number(-5.0, 2, NumberFormat::US), "-5");
    }

    #[test]
    fn test_format_decimal() {
        assert_eq!(format_number(3.14, 2, NumberFormat::US), "3.14");
        assert_eq!(format_number(0.5, 2, NumberFormat::US), "0.5");
        assert_eq!(format_number(1.10, 2, NumberFormat::US), "1.1");
    }

    #[test]
    fn test_format_currency() {
        assert_eq!(format_currency_with_precision(10.0, "$%@", Some(2), NumberFormat::US), "$10");
        assert_eq!(format_currency_with_precision(86.83, "\u{20AC} %@", Some(2), NumberFormat::US), "\u{20AC} 86.83");
        assert_eq!(format_currency_with_precision(150.0, "%@ SFr.", Some(2), NumberFormat::US), "150 SFr.");
    }

    #[test]
    fn test_format_indian() {
        assert_eq!(format_number(1234567.0, 2, NumberFormat::Indian), "12,34,567");
        assert_eq!(format_number(50000.0, 2, NumberFormat::Indian), "50,000");
        assert_eq!(format_number(1234567.89, 2, NumberFormat::Indian), "12,34,567.89");
    }

    #[test]
    fn test_format_european() {
        assert_eq!(format_number(1234567.0, 2, NumberFormat::European), "1.234.567");
        assert_eq!(format_number(1234567.89, 2, NumberFormat::European), "1.234.567,89");
        assert_eq!(format_number(5000.0, 2, NumberFormat::European), "5.000");
    }
}
