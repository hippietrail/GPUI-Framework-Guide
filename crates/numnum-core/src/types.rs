use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CurrencyId(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    Length,
    Mass,
    Time,
    Temperature,
    Area,
    Volume,
    Data,
    Angular,
    Typography,
}

#[derive(Debug, Clone)]
pub struct UnitDef {
    pub canonical: String,
    pub display: String,
    pub dimension: Dimension,
    pub to_base: f64,
    pub offset: f64, // for temperature: value_base = value * to_base + offset
}

#[derive(Debug, Clone)]
pub struct CurrencyDef {
    pub code: String,
    pub display_format: String, // e.g. "$%@" where %@ is the number
    pub rate_to_usd: f64,
}

#[derive(Debug, Clone)]
pub struct UnitTable {
    pub units: Vec<UnitDef>,
    pub name_to_id: HashMap<String, UnitId>,
}

#[derive(Debug, Clone)]
pub struct CurrencyTable {
    pub currencies: Vec<CurrencyDef>,
    pub name_to_id: HashMap<String, CurrencyId>,
}

impl Default for UnitTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CurrencyTable {
    fn default() -> Self {
        Self::new()
    }
}

impl UnitTable {
    pub fn new() -> Self {
        let mut table = UnitTable { units: Vec::new(), name_to_id: HashMap::new() };
        table.build();
        table
    }

    fn add(&mut self, canonical: &str, display: &str, dim: Dimension, to_base: f64, offset: f64, variants: &[&str]) {
        debug_assert!(self.units.len() < u16::MAX as usize, "Too many units for u16 id");
        let id = UnitId(self.units.len() as u16);
        self.units.push(UnitDef {
            canonical: canonical.to_string(),
            display: display.to_string(),
            dimension: dim,
            to_base,
            offset,
        });
        for v in variants {
            self.name_to_id.insert(v.to_lowercase(), id);
        }
    }

    fn add_si_length(&mut self, prefix: &str, symbol: &str, factor: f64) {
        let name = format!("{}meter", prefix);
        let display = format!("{}m", symbol);
        let variants: Vec<String> = vec![
            format!("{}meter", prefix), format!("{}metre", prefix),
            format!("{}meters", prefix), format!("{}metres", prefix),
            format!("{}m", symbol),
        ];
        let v_refs: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
        self.add(&name, &display, Dimension::Length, factor, 0.0, &v_refs);
    }

    fn add_si_mass(&mut self, prefix: &str, symbol: &str, factor: f64) {
        let name = format!("{}gram", prefix);
        let display = format!("{}g", symbol);
        let variants: Vec<String> = vec![
            format!("{}gram", prefix), format!("{}gramme", prefix),
            format!("{}grams", prefix), format!("{}grammes", prefix),
            format!("{}g", symbol),
        ];
        let v_refs: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
        self.add(&name, &display, Dimension::Mass, factor * 0.001, 0.0, &v_refs); // base is kg
    }

    fn add_si_time(&mut self, prefix: &str, symbol: &str, factor: f64) {
        let name = format!("{}second", prefix);
        let display = format!("{}s", symbol);
        let variants: Vec<String> = vec![
            format!("{}second", prefix), format!("{}seconds", prefix),
            format!("{}sec", prefix), format!("{}s", symbol),
        ];
        let v_refs: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
        self.add(&name, &display, Dimension::Time, factor, 0.0, &v_refs);
    }

    fn add_si_data(&mut self, prefix: &str, symbol: &str, factor: f64, is_byte: bool) {
        if is_byte {
            let name = format!("{}byte", prefix);
            let display = format!("{}B", symbol);
            let variants: Vec<String> = vec![
                format!("{}byte", prefix), format!("{}bytes", prefix),
                format!("{}B", symbol),
            ];
            let v_refs: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
            self.add(&name, &display, Dimension::Data, factor * 8.0, 0.0, &v_refs); // base is bit
        } else {
            let name = format!("{}bit", prefix);
            let display = format!("{}b", symbol);
            let variants: Vec<String> = vec![
                format!("{}bit", prefix), format!("{}bits", prefix),
                format!("{}b", symbol),
            ];
            let v_refs: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
            self.add(&name, &display, Dimension::Data, factor, 0.0, &v_refs);
        }
    }

    fn build(&mut self) {
        self.build_length_units();
        self.build_mass_units();
        self.build_time_units();
        self.build_temperature_units();
        self.build_area_units();
        self.build_volume_units();
        self.build_data_units();
        self.build_angular_units();
        self.build_typography_units();
    }

    fn build_length_units(&mut self) {
        let si_prefixes: &[(&str, &str, f64)] = &[
            ("", "", 1.0), ("kilo", "k", 1e3), ("centi", "c", 0.01),
            ("milli", "m", 0.001), ("micro", "u", 1e-6), ("nano", "n", 1e-9),
            ("pico", "p", 1e-12),
        ];
        for &(prefix, sym, factor) in si_prefixes {
            self.add_si_length(prefix, sym, factor);
        }
        self.add("inch", "\u{2033}", Dimension::Length, 0.0254, 0.0,
            &["inch", "inches", "\u{2033}"]);
        self.add("foot", "ft", Dimension::Length, 0.3048, 0.0,
            &["foot", "feet", "ft", "ft.", "foots", "feets"]);
        self.add("yard", "yd", Dimension::Length, 0.9144, 0.0,
            &["yard", "yards", "yd", "yd."]);
        self.add("mile", "mi.", Dimension::Length, 1609.344, 0.0,
            &["mile", "miles", "mi", "mi."]);
        self.add("nautical_mile", "n.m.", Dimension::Length, 1852.0, 0.0,
            &["nautical mile", "nautical miles", "nmi", "n.m."]);
        self.add("chain", "chain", Dimension::Length, 20.1168, 0.0, &["chain", "chains"]);
        self.add("furlong", "furlong", Dimension::Length, 201.168, 0.0, &["furlong", "furlongs"]);
        self.add("league", "league", Dimension::Length, 4828.032, 0.0, &["league", "leagues"]);
        self.add("rod", "rod", Dimension::Length, 5.0292, 0.0, &["rod", "rods"]);
        self.add("cable", "cable", Dimension::Length, 185.2, 0.0, &["cable", "cables"]);
        self.add("hand", "hand", Dimension::Length, 0.1016, 0.0, &["hand", "hands"]);
    }

    fn build_mass_units(&mut self) {
        let mass_si: &[(&str, &str, f64)] = &[
            ("", "", 1.0), ("kilo", "k", 1e3), ("milli", "m", 0.001),
            ("micro", "u", 1e-6),
        ];
        for &(prefix, sym, factor) in mass_si {
            self.add_si_mass(prefix, sym, factor);
        }
        self.add("tonne", "t", Dimension::Mass, 1000.0, 0.0,
            &["tonne", "tonnes", "t", "metric ton"]);
        self.add("pound", "lb", Dimension::Mass, 0.45359237, 0.0,
            &["pound", "pounds", "lb", "lbm"]);
        self.add("ounce", "oz", Dimension::Mass, 0.02834952, 0.0,
            &["ounce", "ounces", "oz"]);
        self.add("stone", "st", Dimension::Mass, 6.35029318, 0.0,
            &["stone", "stones", "st"]);
        self.add("carat", "ct", Dimension::Mass, 0.0002, 0.0,
            &["carat", "carats", "ct"]);
    }

    fn build_time_units(&mut self) {
        let time_si: &[(&str, &str, f64)] = &[
            ("", "", 1.0), ("milli", "m", 0.001),
        ];
        for &(prefix, sym, factor) in time_si {
            self.add_si_time(prefix, sym, factor);
        }
        self.add("minute", "min", Dimension::Time, 60.0, 0.0,
            &["minute", "minutes", "min"]);
        self.add("hour", "h", Dimension::Time, 3600.0, 0.0,
            &["hour", "hours", "h", "hr"]);
        self.add("day", "day", Dimension::Time, 86400.0, 0.0,
            &["day", "days", "d"]);
        self.add("week", "w", Dimension::Time, 604800.0, 0.0,
            &["week", "weeks", "w"]);
        self.add("month", "mon.", Dimension::Time, 2592000.0, 0.0, // 30 days
            &["month", "months", "monthes", "mon", "mon."]);
        self.add("year", "yr", Dimension::Time, 31536000.0, 0.0, // 365 days
            &["year", "years", "yr", "y"]);
    }

    fn build_temperature_units(&mut self) {
        // Base: Kelvin. C = K - 273.15, F = (K - 273.15) * 9/5 + 32
        // to_base converts TO kelvin: K = value * to_base + offset
        self.add("celsius", "\u{00B0}C", Dimension::Temperature, 1.0, 273.15,
            &["celsius", "c", "\u{00B0}c"]);
        self.add("fahrenheit", "\u{00B0}F", Dimension::Temperature, 5.0/9.0, 273.15 - 32.0 * 5.0/9.0,
            &["fahrenheit", "fahrenheits", "f", "\u{00B0}f"]);
        self.add("kelvin", "K", Dimension::Temperature, 1.0, 0.0,
            &["kelvin", "kelvins"]);
        // NOTE: "K" uppercase is Kelvin but conflicts with kilo -- handled in lexer priority
    }

    fn build_area_units(&mut self) {
        self.add("hectare", "ha", Dimension::Area, 10000.0, 0.0,
            &["hectare", "hectares", "ha"]);
        self.add("acre", "acre", Dimension::Area, 4046.8564224, 0.0,
            &["acre", "acres"]);
        self.add("sq_m", "m\u{00B2}", Dimension::Area, 1.0, 0.0,
            &["sq m", "sqm", "square meter", "square metre", "square meters", "m\u{00B2}"]);
        self.add("sq_km", "km\u{00B2}", Dimension::Area, 1e6, 0.0,
            &["sq km", "sqkm", "square kilometer", "square kilometre", "km\u{00B2}"]);
        self.add("sq_ft", "ft\u{00B2}", Dimension::Area, 0.09290304, 0.0,
            &["sq ft", "sqft", "square foot", "square feet", "ft\u{00B2}"]);
        self.add("sq_in", "\u{2033}\u{00B2}", Dimension::Area, 0.00064516, 0.0,
            &["sq inch", "sq inches", "sq in", "square inch", "square inches"]);
        self.add("sq_mi", "mi.\u{00B2}", Dimension::Area, 2589988.11, 0.0,
            &["sq mile", "sq miles", "sq mi", "square mile", "square miles"]);
        self.add("sq_cm", "cm\u{00B2}", Dimension::Area, 0.0001, 0.0,
            &["sq cm", "sqcm", "square centimeter", "square centimetre", "cm\u{00B2}"]);
    }

    fn build_volume_units(&mut self) {
        self.add("liter", "L", Dimension::Volume, 1.0, 0.0,
            &["liter", "litre", "liters", "litres", "l"]);
        self.add("milliliter", "mL", Dimension::Volume, 0.001, 0.0,
            &["milliliter", "millilitre", "ml"]);
        self.add("gallon", "gal.", Dimension::Volume, 3.78541, 0.0,
            &["gallon", "gallons", "gal", "gal."]);
        self.add("quart", "qt.", Dimension::Volume, 0.946353, 0.0,
            &["quart", "quarts", "qt", "qt."]);
        self.add("pint", "pint", Dimension::Volume, 0.473176, 0.0,
            &["pint", "pints"]);
        self.add("cup", "cup", Dimension::Volume, 0.236588, 0.0,
            &["cup", "cups"]);
        self.add("tablespoon", "tbsp.", Dimension::Volume, 0.0147868, 0.0,
            &["tablespoon", "tablespoons", "table spoon", "table spoons", "tbsp", "tbsp."]);
        self.add("teaspoon", "tsp.", Dimension::Volume, 0.00492892, 0.0,
            &["teaspoon", "teaspoons", "tea spoon", "tea spoons", "tsp", "tsp."]);
    }

    fn build_data_units(&mut self) {
        let data_si: &[(&str, &str, f64)] = &[
            ("", "", 1.0), ("kilo", "k", 1e3), ("mega", "M", 1e6),
            ("giga", "G", 1e9), ("tera", "T", 1e12),
        ];
        for &(prefix, sym, factor) in data_si {
            self.add_si_data(prefix, sym, factor, true);  // bytes
            self.add_si_data(prefix, sym, factor, false); // bits
        }
        // IEC binary prefixes (bytes only)
        let iec: &[(&str, &str, f64)] = &[
            ("kibi", "Ki", 1024.0), ("mebi", "Mi", 1048576.0),
            ("gibi", "Gi", 1073741824.0), ("tebi", "Ti", 1099511627776.0),
        ];
        for &(prefix, sym, factor) in iec {
            let name = format!("{}byte", prefix);
            let display = format!("{}B", sym);
            let variants = [format!("{}B", sym)];
            let v_refs: Vec<&str> = variants.iter().map(|s| s.as_str()).collect();
            self.add(&name, &display, Dimension::Data, factor * 8.0, 0.0, &v_refs);
        }
    }

    fn build_angular_units(&mut self) {
        self.add("degree", "\u{00B0}", Dimension::Angular, std::f64::consts::PI / 180.0, 0.0,
            &["degree", "degrees", "\u{00B0}", "deg"]);
        self.add("radian", "rad", Dimension::Angular, 1.0, 0.0,
            &["radian", "radians", "rad"]);
    }

    fn build_typography_units(&mut self) {
        self.add("pixel", "px", Dimension::Typography, 1.0, 0.0,
            &["pixel", "pixels", "px", "px."]);
        self.add("point", "pt", Dimension::Typography, 96.0 / 72.0, 0.0,
            &["point", "points", "pt", "pt."]);
        self.add("em", "em", Dimension::Typography, 16.0, 0.0,
            &["em", "ems"]);
    }

    pub fn lookup(&self, name: &str) -> Option<UnitId> {
        self.name_to_id.get(&name.to_lowercase()).copied()
    }

    pub fn get(&self, id: UnitId) -> Option<&UnitDef> {
        self.units.get(id.0 as usize)
    }

    pub fn convert(&self, value: f64, from: UnitId, to: UnitId) -> Option<f64> {
        let from_def = self.get(from)?;
        let to_def = self.get(to)?;
        if from_def.dimension != to_def.dimension {
            return None;
        }
        // Convert to base, then from base to target
        let base_value = value * from_def.to_base + from_def.offset;
        let result = (base_value - to_def.offset) / to_def.to_base;
        Some(result)
    }
}

impl CurrencyTable {
    pub fn new() -> Self {
        let mut table = CurrencyTable { currencies: Vec::new(), name_to_id: HashMap::new() };
        table.build();
        table
    }

    fn add(&mut self, code: &str, display: &str, rate: f64, variants: &[&str]) {
        debug_assert!(self.currencies.len() < u16::MAX as usize, "Too many currencies for u16 id");
        let id = CurrencyId(self.currencies.len() as u16);
        self.currencies.push(CurrencyDef {
            code: code.to_string(),
            display_format: display.to_string(),
            rate_to_usd: rate,
        });
        self.name_to_id.insert(code.to_lowercase(), id);
        for v in variants {
            self.name_to_id.insert(v.to_lowercase(), id);
        }
    }

    fn build(&mut self) {
        // Static rates (snapshot -- live rates would come from API)
        self.add("USD", "$%@", 1.0, &["dollar", "dollars", "dollar usa", "dollars usa",
            "u.s. dollar", "u.s. dollars", "usa dollar", "usa dollars"]);
        self.add("EUR", "\u{20AC} %@", 0.87, &["euro", "euros"]);
        self.add("GBP", "\u{00A3} %@", 0.76, &["pound sterling", "pounds sterling",
            "british pound", "british pounds"]);
        self.add("JPY", "\u{00A5} %@", 149.5, &["yen", "japanese yen"]);
        self.add("CHF", "%@ SFr.", 0.88, &["swiss franc", "swiss francs"]);
        self.add("CAD", "$%@ CAD", 1.36, &["canadian dollar", "canadian dollars"]);
        self.add("AUD", "$%@ AUD", 1.53, &["australian dollar", "australian dollars"]);
        self.add("CNY", "%@ \u{00A5}", 7.24, &["yuan", "yuans", "chinese yuan"]);
        self.add("INR", "INR %@", 83.5, &["indian rupee", "rupee"]);
        self.add("KRW", "\u{20A9} %@", 1330.0, &["won", "south korean won"]);
        self.add("RUB", "%@ \u{20BD}", 92.0, &["ruble", "rubles", "rouble", "roubles",
            "russian ruble", "russian rubles"]);
        self.add("BRL", "R$ %@", 5.0, &["brazilian real", "brazilian reals"]);
        self.add("MXN", "$ %@", 17.2, &["mexican peso", "mexican pesos"]);
        self.add("SEK", "%@ kr", 10.5, &["swedish krona", "swedish kronor"]);
        self.add("NOK", "%@ kr", 10.6, &["norwegian krone", "norwegian kroner"]);
        self.add("DKK", "%@ kr.", 6.9, &["danish krone", "danish kroner"]);
        self.add("PLN", "%@ z\u{0142}", 4.0, &["zloty", "polish zloty"]);
        self.add("CZK", "%@ K\u{010D}", 23.0, &["czech koruna", "czech korunas"]);
        self.add("HUF", "%@ Ft", 360.0, &["forint", "hungarian forint"]);
        self.add("TRY", "\u{20BA}%@", 27.0, &["turkish lira"]);
        self.add("SGD", "S$ %@", 1.35, &["singapore dollar", "singapore dollars"]);
        self.add("HKD", "HK$ %@", 7.82, &["hong kong dollar", "hong kong dollars"]);
        self.add("THB", "\u{0E3F} %@", 35.5, &["baht", "thai baht"]);
        self.add("ZAR", "R %@", 18.5, &["rand", "south african rand"]);
        self.add("AED", "%@ Dh", 3.67, &["dirham", "dirhams", "uae dirham"]);
        self.add("SAR", "%@ SR", 3.75, &["saudi riyal", "saudi riyals"]);
        self.add("ILS", "%@ NIS", 3.7, &["shekel", "new shekel", "israeli new shekel"]);
        self.add("UAH", "%@ UAH", 37.5, &["hryvnia", "ukrainian hryvnia"]);
        self.add("NZD", "$%@ NZD", 1.63, &["new zealand dollar", "new zealand dollars"]);
        // Crypto
        self.add("BTC", "%@ BTC", 0.000015, &["bitcoin", "bitcoins"]);
        self.add("ETH", "%@ ETH", 0.00049, &["ethereum", "ether"]);
    }

    pub fn lookup(&self, name: &str) -> Option<CurrencyId> {
        self.name_to_id.get(&name.to_lowercase()).copied()
    }

    pub fn get(&self, id: CurrencyId) -> Option<&CurrencyDef> {
        self.currencies.get(id.0 as usize)
    }

    pub fn convert(&self, value: f64, from: CurrencyId, to: CurrencyId) -> Option<f64> {
        let from_rate = self.get(from)?.rate_to_usd;
        let to_rate = self.get(to)?.rate_to_usd;
        Some(value * to_rate / from_rate)
    }
}

/// How a number was originally entered — affects display format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumRepr {
    Decimal,
    Hex,
    Binary,
    Octal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    NumberRepr(f64, NumRepr), // number with explicit display format
    WithUnit(f64, UnitId),
    WithCurrency(f64, CurrencyId),
    Percent(f64), // stored as fraction: 50% = 0.5
    None,
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) | Value::NumberRepr(n, _) => Some(*n),
            Value::WithUnit(n, _) => Some(*n),
            Value::WithCurrency(n, _) => Some(*n),
            Value::Percent(n) => Some(*n * 100.0),
            Value::None => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", crate::format::format_number(*n)),
            Value::NumberRepr(n, repr) => write!(f, "{}", crate::format::format_number_repr(*n, *repr)),
            Value::WithUnit(n, _) => write!(f, "{}", crate::format::format_number(*n)),
            Value::WithCurrency(n, _) => write!(f, "{}", crate::format::format_number(*n)),
            Value::Percent(n) => write!(f, "{} %", crate::format::format_number(*n * 100.0)),
            Value::None => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuncKind {
    Sqrt, Cbrt, Abs, Round, Ceil, Floor,
    Log, Ln, Fact,
    Sin, Cos, Tan, Asin, Acos, Atan,
    Sinh, Cosh, Tanh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReprKind {
    Hex, Binary, Octal, Decimal, Scientific,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Pow,
    BitAnd, BitOr, BitXor, Shl, Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOp {
    AddAssign, SubAssign, MulAssign, DivAssign,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    NumberRepr(f64, NumRepr),
    BinaryOp { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr> },
    UnaryMinus(Box<Expr>),
    Variable(String),
    Assignment { name: String, value: Box<Expr> },
    CompoundAssignment { name: String, op: CompoundOp, value: Box<Expr> },
    FunctionCall { func: FuncKind, arg: Box<Expr> },
    WithUnit { expr: Box<Expr>, unit: UnitId },
    WithCurrency { expr: Box<Expr>, currency: CurrencyId },
    Conversion { expr: Box<Expr>, target: ConversionTarget },
    PercentOf { pct: Box<Expr>, base: Box<Expr> },
    PercentOn { pct: Box<Expr>, base: Box<Expr> },
    PercentOff { pct: Box<Expr>, base: Box<Expr> },
    InlinePercentAdd { base: Box<Expr>, pct: Box<Expr> },
    InlinePercentSub { base: Box<Expr>, pct: Box<Expr> },
    ReversePercentOf { pct: Box<Expr>, result: Box<Expr> },
    ReversePercentOn { pct: Box<Expr>, result: Box<Expr> },
    ReversePercentOff { pct: Box<Expr>, result: Box<Expr> },
    AsAPercentOf { value: Box<Expr>, base: Box<Expr> },
    AsAPercentOn { value: Box<Expr>, base: Box<Expr> },
    AsAPercentOff { value: Box<Expr>, base: Box<Expr> },
    Percent(Box<Expr>), // bare N%
    Aggregation(AggKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggKind { Sum, Average, Prev }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionTarget {
    Unit(UnitId),
    Currency(CurrencyId),
    Repr(ReprKind),
}
