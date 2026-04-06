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
        self.add("inch", "inches", Dimension::Length, 0.0254, 0.0,
            &["inch", "inches", "in", "\u{2033}"]);
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
        // Rate = how many units of this currency per 1 USD

        // --- Major world currencies ---
        self.add("USD", "$%@", 1.0, &["dollar", "dollars", "dollar usa", "dollars usa",
            "u.s. dollar", "u.s. dollars", "usa dollar", "usa dollars",
            "usd", "bucks"]);
        self.add("EUR", "\u{20AC} %@", 0.87, &["euro", "euros", "eur"]);
        self.add("GBP", "\u{00A3} %@", 0.76, &["pound sterling", "pounds sterling",
            "british pound", "british pounds", "gbp", "quid"]);
        self.add("JPY", "\u{00A5} %@", 149.5, &["yen", "japanese yen", "jpy"]);
        self.add("CHF", "%@ SFr.", 0.88, &["swiss franc", "swiss francs", "sfr.", "sfr",
            "chf", "franc", "francs"]);
        self.add("CAD", "$%@ CAD", 1.36, &["canadian dollar", "canadian dollars", "cad"]);
        self.add("AUD", "$%@ AUD", 1.53, &["australian dollar", "australian dollars", "aud"]);
        self.add("CNY", "%@ \u{00A5}", 7.24, &["yuan", "yuans", "chinese yuan",
            "rmb", "renminbi", "cny"]);
        self.add("INR", "\u{20B9} %@", 83.5, &["indian rupee", "rupee", "rupees",
            "inr", "rs", "\u{20B9}"]);
        self.add("KRW", "\u{20A9} %@", 1330.0, &["won", "south korean won", "krw"]);
        self.add("RUB", "%@ \u{20BD}", 92.0, &["ruble", "rubles", "rouble", "roubles",
            "russian ruble", "russian rubles", "rub"]);
        self.add("BRL", "R$ %@", 5.0, &["brazilian real", "brazilian reals", "r$",
            "brl", "real", "reais"]);
        self.add("MXN", "$ %@", 17.2, &["mexican peso", "mexican pesos", "mxn"]);
        self.add("SEK", "%@ kr", 10.5, &["swedish krona", "swedish kronor", "sek"]);
        self.add("NOK", "%@ kr", 10.6, &["norwegian krone", "norwegian kroner", "nok"]);
        self.add("DKK", "%@ kr.", 6.9, &["danish krone", "danish kroner", "dkk"]);
        self.add("PLN", "%@ z\u{0142}", 4.0, &["zloty", "polish zloty", "pln"]);
        self.add("CZK", "%@ K\u{010D}", 23.0, &["czech koruna", "czech korunas", "czk"]);
        self.add("HUF", "%@ Ft", 360.0, &["forint", "hungarian forint", "huf"]);
        self.add("TRY", "\u{20BA}%@", 27.0, &["turkish lira", "lira"]);
        self.add("SGD", "S$ %@", 1.35, &["singapore dollar", "singapore dollars", "s$", "sgd"]);
        self.add("HKD", "HK$ %@", 7.82, &["hong kong dollar", "hong kong dollars", "hk$", "hkd"]);
        self.add("THB", "\u{0E3F} %@", 35.5, &["baht", "thai baht", "thb"]);
        self.add("ZAR", "R %@", 18.5, &["rand", "south african rand", "zar"]);
        self.add("AED", "%@ Dh", 3.67, &["dirham", "dirhams", "uae dirham", "dh", "aed"]);
        self.add("SAR", "%@ SR", 3.75, &["saudi riyal", "saudi riyals", "sr", "sar"]);
        self.add("ILS", "%@ NIS", 3.7, &["shekel", "new shekel", "israeli new shekel", "nis", "ils"]);
        self.add("UAH", "%@ UAH", 37.5, &["hryvnia", "ukrainian hryvnia", "uah"]);
        self.add("NZD", "$%@ NZD", 1.63, &["new zealand dollar", "new zealand dollars", "nzd"]);

        // --- South Asia ---
        self.add("PKR", "\u{20A8} %@", 278.0, &["pakistani rupee", "pakistani rupees", "pkr", "\u{20A8}"]);
        self.add("BDT", "\u{09F3} %@", 110.0, &["bangladeshi taka", "taka", "takas", "bdt", "tk", "\u{09F3}"]);
        self.add("LKR", "Rs %@", 325.0, &["sri lankan rupee", "sri lankan rupees", "lkr"]);
        self.add("NPR", "Rs %@", 133.0, &["nepalese rupee", "nepalese rupees", "npr"]);

        // --- Southeast Asia ---
        self.add("PHP", "\u{20B1} %@", 56.0, &["philippine peso", "philippine pesos", "php", "\u{20B1}"]);
        self.add("IDR", "Rp %@", 15800.0, &["indonesian rupiah", "rupiah", "idr", "rp"]);
        self.add("MYR", "RM %@", 4.5, &["malaysian ringgit", "ringgit", "myr", "rm"]);
        self.add("VND", "%@ \u{20AB}", 25000.0, &["vietnamese dong", "dong", "vnd", "\u{20AB}"]);

        // --- East Asia ---
        self.add("TWD", "NT$ %@", 32.0, &["new taiwan dollar", "new taiwan dollars",
            "twd", "nt$", "nt dollar", "ntd"]);

        // --- Middle East ---
        self.add("KWD", "%@ KD", 0.31, &["kuwaiti dinar", "kuwaiti dinars", "kwd"]);
        self.add("QAR", "%@ QR", 3.64, &["qatari rial", "qatari rials", "qar"]);
        self.add("OMR", "%@ RO", 0.385, &["omani rial", "omani rials", "omr"]);
        self.add("BHD", "%@ BD", 0.376, &["bahraini dinar", "bahraini dinars", "bhd"]);
        self.add("JOD", "%@ JD", 0.709, &["jordanian dinar", "jordanian dinars", "jod"]);
        self.add("EGP", "E\u{00A3} %@", 49.0, &["egyptian pound", "egyptian pounds", "egp"]);

        // --- Africa ---
        self.add("NGN", "\u{20A6} %@", 1550.0, &["nigerian naira", "naira", "ngn", "\u{20A6}"]);
        self.add("KES", "%@ KSh", 154.0, &["kenyan shilling", "kenyan shillings", "kes", "ksh"]);
        self.add("GHS", "GH\u{20B5} %@", 14.5, &["ghanaian cedi", "cedi", "ghs", "gh\u{20B5}"]);
        self.add("TZS", "%@ TSh", 2650.0, &["tanzanian shilling", "tanzanian shillings", "tzs", "tsh"]);

        // --- Europe (additional) ---
        self.add("RON", "%@ lei", 4.6, &["romanian leu", "romanian lei", "ron"]);
        self.add("BGN", "%@ лв", 1.8, &["bulgarian lev", "bulgarian leva", "lev", "leva", "bgn"]);
        self.add("HRK", "%@ Kn", 7.0, &["croatian kuna", "kuna", "hrk"]);
        self.add("ISK", "%@ kr", 138.0, &["icelandic krona", "icelandic kronas", "isk"]);

        // --- South America ---
        self.add("COP", "$%@ COP", 4100.0, &["colombian peso", "colombian pesos", "cop"]);
        self.add("ARS", "%@ ARS", 870.0, &["argentine peso", "argentine pesos", "ars"]);
        self.add("CLP", "%@ CLP", 950.0, &["chilean peso", "chilean pesos", "clp"]);
        self.add("PEN", "%@ S/.", 3.75, &["peruvian sol", "peruvian soles", "pen", "s/."]);
        self.add("UYU", "$U %@", 40.0, &["uruguayan peso", "uruguayan pesos", "uyu", "$u"]);

        // --- Central Asia ---
        self.add("KGS", "%@ KGS", 87.447949, &["kyrgyz som", "kyrgyzstani som", "kgs"]);
        self.add("KZT", "\u{20B8} %@", 472.588461, &["kazakhstani tenge", "tenge", "kzt", "\u{20B8}"]);
        self.add("TJS", "%@ TJS", 9.557188, &["tajikistani somoni", "somoni", "tjs"]);
        self.add("TMT", "%@ TMT", 3.500314, &["turkmenistani manat", "turkmen manat", "tmt"]);
        self.add("UZS", "%@ UZS", 12194.618803, &["uzbekistani som", "uzbek som", "uzs"]);
        self.add("AZN", "\u{20BC} %@", 1.700131, &["azerbaijani manat", "manat", "azn", "\u{20BC}"]);
        self.add("GEL", "\u{20BE} %@", 2.695614, &["georgian lari", "lari", "gel", "\u{20BE}"]);
        self.add("AMD", "\u{058F} %@", 377.151954, &["armenian dram", "dram", "amd", "\u{058F}"]);

        // --- South Asia (additional) ---
        self.add("BTN", "Nu. %@", 93.181617, &["bhutanese ngultrum", "ngultrum", "btn"]);
        self.add("MVR", "Rf %@", 15.449077, &["maldivian rufiyaa", "rufiyaa", "mvr"]);
        self.add("AFN", "Af %@", 64.60017, &["afghan afghani", "afghani", "afn", "af"]);

        // --- Southeast Asia (additional) ---
        self.add("KHR", "\u{17DB} %@", 3992.89703, &["cambodian riel", "riel", "khr", "\u{17DB}"]);
        self.add("LAK", "\u{20AD} %@", 21969.844376, &["lao kip", "laotian kip", "kip", "lak", "\u{20AD}"]);
        self.add("MMK", "K %@", 2101.122717, &["myanmar kyat", "burmese kyat", "kyat", "mmk"]);
        self.add("BND", "B$ %@", 1.286393, &["brunei dollar", "brunei dollars", "bnd"]);

        // --- East Asia (additional) ---
        self.add("MOP", "MOP$ %@", 8.07286, &["macanese pataca", "pataca", "mop"]);
        self.add("MNT", "\u{20AE} %@", 3589.942392, &["mongolian tugrik", "tugrik", "togrog", "mnt", "\u{20AE}"]);
        self.add("CNH", "%@ CNH", 6.885876, &["offshore yuan", "offshore renminbi", "cnh"]);

        // --- Middle East (additional) ---
        self.add("IQD", "%@ IQD", 1311.095307, &["iraqi dinar", "iraqi dinars", "iqd"]);
        self.add("IRR", "%@ IRR", 1131435.415551, &["iranian rial", "iranian rials", "irr"]);
        self.add("LBP", "L\u{00A3} %@", 89500.0, &["lebanese pound", "lebanese pounds", "lbp"]);
        self.add("SYP", "\u{00A3}S %@", 113.971124, &["syrian pound", "syrian pounds", "syp"]);
        self.add("YER", "%@ YER", 238.584256, &["yemeni rial", "yemeni rials", "yer"]);

        // --- Africa (additional) ---
        self.add("DZD", "%@ DZD", 133.090473, &["algerian dinar", "algerian dinars", "dzd"]);
        self.add("AOA", "%@ AOA", 920.402247, &["angolan kwanza", "kwanza", "aoa"]);
        self.add("BWP", "P %@", 14.063811, &["botswana pula", "pula", "bwp"]);
        self.add("BIF", "%@ BIF", 2980.867789, &["burundian franc", "burundi franc", "bif"]);
        self.add("CVE", "%@ CVE", 95.654455, &["cape verdean escudo", "cape verde escudo", "cve"]);
        self.add("CDF", "%@ CDF", 2293.584609, &["congolese franc", "congo franc", "cdf"]);
        self.add("DJF", "%@ DJF", 177.721, &["djiboutian franc", "djibouti franc", "djf"]);
        self.add("ERN", "%@ ERN", 15.0, &["eritrean nakfa", "nakfa", "ern"]);
        self.add("ETB", "%@ ETB", 155.519206, &["ethiopian birr", "birr", "etb"]);
        self.add("GMD", "%@ GMD", 74.214935, &["gambian dalasi", "dalasi", "gmd"]);
        self.add("GNF", "%@ GNF", 8772.851031, &["guinean franc", "guinea franc", "gnf"]);
        self.add("KMF", "%@ KMF", 426.780096, &["comorian franc", "comoros franc", "kmf"]);
        self.add("LRD", "L$ %@", 183.491951, &["liberian dollar", "liberian dollars", "lrd"]);
        self.add("LSL", "%@ LSL", 16.986342, &["lesotho loti", "loti", "maloti", "lsl"]);
        self.add("LYD", "%@ LYD", 6.389797, &["libyan dinar", "libyan dinars", "lyd"]);
        self.add("MAD", "%@ MAD", 9.366464, &["moroccan dirham", "moroccan dirhams", "mad"]);
        self.add("MGA", "%@ MGA", 4171.56798, &["malagasy ariary", "ariary", "mga"]);
        self.add("MKD", "%@ MKD", 53.454119, &["macedonian denar", "denar", "denari", "mkd"]);
        self.add("MRU", "%@ MRU", 40.08329, &["mauritanian ouguiya", "ouguiya", "mru"]);
        self.add("MUR", "\u{20A8} %@ MUR", 46.846577, &["mauritian rupee", "mauritian rupees", "mur"]);
        self.add("MWK", "MK %@", 1744.20424, &["malawian kwacha", "malawi kwacha", "mwk"]);
        self.add("MZN", "%@ MZN", 63.706996, &["mozambican metical", "metical", "meticais", "mzn"]);
        self.add("NAD", "N$ %@", 16.986342, &["namibian dollar", "namibian dollars", "nad"]);
        self.add("RWF", "%@ RWF", 1457.974283, &["rwandan franc", "rwanda franc", "rwf"]);
        self.add("SCR", "\u{20A8} %@ SCR", 14.385468, &["seychellois rupee", "seychelles rupee", "scr"]);
        self.add("SDG", "%@ SDG", 508.997922, &["sudanese pound", "sudanese pounds", "sdg"]);
        self.add("SHP", "\u{00A3} %@ SHP", 0.75732, &["saint helena pound", "st helena pound", "shp"]);
        self.add("SLE", "Le %@", 24.661731, &["sierra leonean leone", "leone", "sle"]);
        self.add("SLL", "Le %@ SLL", 24661.730761, &["sierra leonean leone old", "old leone", "sll"]);
        self.add("SOS", "Sh %@", 571.610021, &["somali shilling", "somali shillings", "sos"]);
        self.add("SSP", "\u{00A3} %@ SSP", 4588.612193, &["south sudanese pound", "south sudan pound", "ssp"]);
        self.add("STN", "%@ STN", 21.253654, &["sao tome dobra", "dobra", "stn"]);
        self.add("SZL", "E %@", 16.986342, &["swazi lilangeni", "lilangeni", "emalangeni", "szl"]);
        self.add("UGX", "USh %@", 3754.257525, &["ugandan shilling", "ugandan shillings", "ugx"]);
        self.add("XAF", "%@ FCFA", 569.040128, &["central african cfa franc", "cfa franc beac", "xaf"]);
        self.add("XOF", "%@ CFA", 569.040128, &["west african cfa franc", "cfa franc bceao", "xof"]);
        self.add("ZMW", "ZK %@", 19.320298, &["zambian kwacha", "zambia kwacha", "zmw"]);
        self.add("ZWG", "%@ ZWG", 25.3745, &["zimbabwe gold", "zig", "zwg"]);
        self.add("ZWL", "Z$ %@", 25.3745, &["zimbabwean dollar", "zimbabwean dollars", "zwl"]);

        // --- Europe (additional) ---
        self.add("ALL", "%@ ALL", 83.059825, &["albanian lek", "lek", "leke", "all"]);
        self.add("BAM", "KM %@", 1.696675, &["bosnia mark", "convertible mark", "bosnian mark", "bam"]);
        self.add("BYN", "Br %@", 2.964765, &["belarusian ruble", "belarusian rubles", "byn"]);
        self.add("GGP", "\u{00A3} %@ GGP", 0.75732, &["guernsey pound", "ggp"]);
        self.add("GIP", "\u{00A3} %@ GIP", 0.75732, &["gibraltar pound", "gibraltar pounds", "gip"]);
        self.add("IMP", "\u{00A3} %@ IMP", 0.75732, &["manx pound", "isle of man pound", "imp"]);
        self.add("JEP", "\u{00A3} %@ JEP", 0.75732, &["jersey pound", "jep"]);
        self.add("MDL", "%@ MDL", 17.589358, &["moldovan leu", "moldovan lei", "mdl"]);
        self.add("RSD", "%@ RSD", 101.744411, &["serbian dinar", "serbian dinars", "rsd"]);
        self.add("FOK", "%@ FOK", 6.471317, &["faroese krona", "faroe krona", "fok"]);

        // --- North Africa / Mediterranean ---
        self.add("TND", "%@ TND", 2.931721, &["tunisian dinar", "tunisian dinars", "tnd"]);

        // --- South America (additional) ---
        self.add("BOB", "Bs %@", 6.926047, &["boliviano", "bolivianos", "bob"]);
        self.add("GYD", "G$ %@", 209.358988, &["guyanese dollar", "guyanese dollars", "gyd"]);
        self.add("PYG", "\u{20B2} %@", 6510.656336, &["paraguayan guarani", "guarani", "pyg", "\u{20B2}"]);
        self.add("SRD", "Sr$ %@", 37.458656, &["surinamese dollar", "suriname dollar", "srd"]);
        self.add("VES", "Bs.S %@", 474.0598, &["venezuelan bolivar", "bolivar soberano", "ves"]);
        self.add("CLF", "%@ CLF", 0.023227, &["chilean unit of account", "unidad de fomento", "clf"]);

        // --- Central America & Caribbean ---
        self.add("BBD", "Bds$ %@", 2.0, &["barbadian dollar", "barbados dollar", "bbd"]);
        self.add("BMD", "BD$ %@", 1.0, &["bermudian dollar", "bermuda dollar", "bmd"]);
        self.add("BSD", "B$ %@ BSD", 1.0, &["bahamian dollar", "bahamas dollar", "bsd"]);
        self.add("BZD", "BZ$ %@", 2.0, &["belize dollar", "belize dollars", "bzd"]);
        self.add("CRC", "\u{20A1} %@", 465.35897, &["costa rican colon", "colon", "colones", "crc", "\u{20A1}"]);
        self.add("CUP", "$MN %@", 24.0, &["cuban peso", "cuban pesos", "cup"]);
        self.add("DOP", "RD$ %@", 60.358559, &["dominican peso", "dominican pesos", "dop"]);
        self.add("GTQ", "Q %@", 7.651701, &["guatemalan quetzal", "quetzal", "quetzales", "gtq"]);
        self.add("HNL", "L %@", 26.581894, &["honduran lempira", "lempira", "lempiras", "hnl"]);
        self.add("HTG", "G %@", 130.940308, &["haitian gourde", "gourde", "htg"]);
        self.add("JMD", "J$ %@", 157.869373, &["jamaican dollar", "jamaican dollars", "jmd"]);
        self.add("KYD", "CI$ %@", 0.833333, &["cayman islands dollar", "cayman dollar", "kyd"]);
        self.add("NIO", "C$ %@", 36.808026, &["nicaraguan cordoba", "cordoba", "cordobas", "nio"]);
        self.add("PAB", "B/. %@", 1.0, &["panamanian balboa", "balboa", "balboas", "pab"]);
        self.add("TTD", "TT$ %@", 6.766397, &["trinidad dollar", "trinidad and tobago dollar", "ttd"]);
        self.add("XCD", "EC$ %@", 2.7, &["east caribbean dollar", "ec dollar", "xcd"]);
        self.add("ANG", "\u{0192} %@", 1.79, &["netherlands antillean guilder", "antillean guilder", "ang", "\u{0192}"]);
        self.add("AWG", "\u{0192} %@ AWG", 1.79, &["aruban florin", "aruba florin", "awg"]);
        self.add("XCG", "%@ XCG", 1.79, &["caribbean guilder", "xcg"]);

        // --- Pacific ---
        self.add("FJD", "FJ$ %@", 2.255663, &["fijian dollar", "fiji dollar", "fjd"]);
        self.add("PGK", "K %@ PGK", 4.323403, &["papua new guinean kina", "kina", "pgk"]);
        self.add("SBD", "SI$ %@", 7.950994, &["solomon islands dollar", "solomon dollar", "sbd"]);
        self.add("TOP", "T$ %@", 2.384648, &["tongan paanga", "paanga", "top"]);
        self.add("TVD", "TV$ %@", 1.44996, &["tuvaluan dollar", "tuvalu dollar", "tvd"]);
        self.add("VUV", "VT %@", 119.19378, &["vanuatu vatu", "vatu", "vuv"]);
        self.add("WST", "WS$ %@", 2.7315, &["samoan tala", "tala", "wst"]);
        self.add("KID", "A$ %@ KID", 1.44996, &["kiribati dollar", "kid"]);
        self.add("XPF", "%@ XPF", 103.520042, &["cfp franc", "pacific franc", "xpf"]);

        // --- Falkland Islands ---
        self.add("FKP", "FK\u{00A3} %@", 0.75732, &["falkland islands pound", "falkland pound", "fkp"]);

        // --- Special Drawing Rights ---
        self.add("XDR", "%@ XDR", 0.735298, &["special drawing rights", "sdr", "xdr"]);

        // --- Crypto ---
        self.add("BTC", "%@ BTC", 0.000015, &["bitcoin", "bitcoins", "btc"]);
        self.add("ETH", "%@ ETH", 0.00049, &["ethereum", "ether", "eth"]);
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

    /// Update the rate for a currency identified by its ISO code.
    /// If the code exists, its `rate_to_usd` is updated and `true` is returned.
    /// If the code is not found, returns `false` (no new currency is inserted).
    pub fn update_rate(&mut self, code: &str, rate_to_usd: f64) -> bool {
        if let Some(&id) = self.name_to_id.get(&code.to_lowercase())
            && let Some(def) = self.currencies.get_mut(id.0 as usize)
        {
            def.rate_to_usd = rate_to_usd;
            return true;
        }
        false
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
            Value::Number(n) => write!(f, "{}", crate::format::format_number(*n, 2)),
            Value::NumberRepr(n, repr) => write!(f, "{}", crate::format::format_number_repr(*n, *repr)),
            Value::WithUnit(n, _) => write!(f, "{}", crate::format::format_number(*n, 2)),
            Value::WithCurrency(n, _) => write!(f, "{}", crate::format::format_number(*n, 2)),
            Value::Percent(n) => write!(f, "{} %", crate::format::format_number(*n * 100.0, 2)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_original_currencies_present() {
        let ct = CurrencyTable::new();
        // All original currencies must still resolve
        let originals = [
            "USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD", "CNY",
            "INR", "KRW", "RUB", "BRL", "MXN", "SEK", "NOK", "DKK",
            "PLN", "CZK", "HUF", "TRY", "SGD", "HKD", "THB", "ZAR",
            "AED", "SAR", "ILS", "UAH", "NZD", "BTC", "ETH",
        ];
        for code in &originals {
            assert!(ct.lookup(&code.to_lowercase()).is_some(),
                "Original currency {} should be present", code);
        }
    }

    #[test]
    fn test_new_major_currencies_present() {
        let ct = CurrencyTable::new();
        let new_currencies = [
            ("PKR", 278.0),
            ("BDT", 110.0),
            ("LKR", 325.0),
            ("NPR", 133.0),
            ("PHP", 56.0),
            ("IDR", 15800.0),
            ("MYR", 4.5),
            ("VND", 25000.0),
            ("TWD", 32.0),
            ("KWD", 0.31),
            ("QAR", 3.64),
            ("OMR", 0.385),
            ("BHD", 0.376),
            ("JOD", 0.709),
            ("EGP", 49.0),
            ("NGN", 1550.0),
            ("KES", 154.0),
            ("GHS", 14.5),
            ("TZS", 2650.0),
            ("RON", 4.6),
            ("BGN", 1.8),
            ("HRK", 7.0),
            ("ISK", 138.0),
            ("COP", 4100.0),
            ("ARS", 870.0),
            ("CLP", 950.0),
            ("PEN", 3.75),
            ("UYU", 40.0),
        ];
        for (code, expected_rate) in &new_currencies {
            let id = ct.lookup(&code.to_lowercase())
                .unwrap_or_else(|| panic!("Currency {} not found", code));
            let def = ct.get(id).unwrap();
            assert_eq!(def.code, *code);
            assert!((def.rate_to_usd - expected_rate).abs() < 0.01,
                "{} rate mismatch: expected {}, got {}", code, expected_rate, def.rate_to_usd);
        }
    }

    #[test]
    fn test_usd_variants() {
        let ct = CurrencyTable::new();
        let usd_id = ct.lookup("usd").unwrap();
        for v in &["dollar", "dollars", "usd", "bucks"] {
            assert_eq!(ct.lookup(v), Some(usd_id), "USD variant '{}' missing", v);
        }
    }

    #[test]
    fn test_eur_variants() {
        let ct = CurrencyTable::new();
        let eur_id = ct.lookup("eur").unwrap();
        for v in &["euro", "euros", "eur"] {
            assert_eq!(ct.lookup(v), Some(eur_id), "EUR variant '{}' missing", v);
        }
    }

    #[test]
    fn test_gbp_variants() {
        let ct = CurrencyTable::new();
        let gbp_id = ct.lookup("gbp").unwrap();
        for v in &["gbp", "quid", "pound sterling", "british pound"] {
            assert_eq!(ct.lookup(v), Some(gbp_id), "GBP variant '{}' missing", v);
        }
    }

    #[test]
    fn test_cny_variants() {
        let ct = CurrencyTable::new();
        let cny_id = ct.lookup("cny").unwrap();
        for v in &["yuan", "rmb", "renminbi", "cny", "chinese yuan"] {
            assert_eq!(ct.lookup(v), Some(cny_id), "CNY variant '{}' missing", v);
        }
    }

    #[test]
    fn test_inr_variants() {
        let ct = CurrencyTable::new();
        let inr_id = ct.lookup("inr").unwrap();
        for v in &["rupee", "rupees", "inr", "rs", "\u{20B9}"] {
            assert_eq!(ct.lookup(v), Some(inr_id), "INR variant '{}' missing", v);
        }
    }

    #[test]
    fn test_chf_variants() {
        let ct = CurrencyTable::new();
        let chf_id = ct.lookup("chf").unwrap();
        for v in &["chf", "franc", "francs", "swiss franc", "swiss francs"] {
            assert_eq!(ct.lookup(v), Some(chf_id), "CHF variant '{}' missing", v);
        }
    }

    #[test]
    fn test_brl_variants() {
        let ct = CurrencyTable::new();
        let brl_id = ct.lookup("brl").unwrap();
        for v in &["brl", "real", "reais", "r$", "brazilian real"] {
            assert_eq!(ct.lookup(v), Some(brl_id), "BRL variant '{}' missing", v);
        }
    }

    #[test]
    fn test_compound_symbol_variants() {
        let ct = CurrencyTable::new();
        // Compound symbols should be in variants so copy-paste works
        assert!(ct.lookup("r$").is_some(), "R$ should resolve to BRL");
        assert!(ct.lookup("hk$").is_some(), "HK$ should resolve to HKD");
        assert!(ct.lookup("s$").is_some(), "S$ should resolve to SGD");
        assert!(ct.lookup("nt$").is_some(), "NT$ should resolve to TWD");
        assert!(ct.lookup("$u").is_some(), "$U should resolve to UYU");
    }

    #[test]
    fn test_currency_conversion_new_currencies() {
        let ct = CurrencyTable::new();
        let usd_id = ct.lookup("usd").unwrap();
        let pkr_id = ct.lookup("pkr").unwrap();
        // 1 USD -> PKR should be ~278
        let result = ct.convert(1.0, usd_id, pkr_id).unwrap();
        assert!((result - 278.0).abs() < 1.0, "1 USD should be ~278 PKR, got {}", result);
    }

    #[test]
    fn test_currency_conversion_cross() {
        let ct = CurrencyTable::new();
        let eur_id = ct.lookup("eur").unwrap();
        let gbp_id = ct.lookup("gbp").unwrap();
        // EUR->GBP: 1 EUR in USD = 1/0.87, then USD->GBP = (1/0.87)*0.76
        let result = ct.convert(1.0, eur_id, gbp_id).unwrap();
        let expected = 0.76 / 0.87;
        assert!((result - expected).abs() < 0.01,
            "1 EUR -> GBP: expected {}, got {}", expected, result);
    }

    #[test]
    fn test_update_rate_new_currency() {
        let mut ct = CurrencyTable::new();
        assert!(ct.update_rate("PKR", 280.0));
        let id = ct.lookup("pkr").unwrap();
        let def = ct.get(id).unwrap();
        assert!((def.rate_to_usd - 280.0).abs() < 0.01);
    }

    #[test]
    fn test_iso_code_lookup_case_insensitive() {
        let ct = CurrencyTable::new();
        // ISO codes registered via variants should work in lowercase
        let codes = [
            "usd", "eur", "gbp", "jpy", "chf", "cad", "aud", "cny",
            "inr", "krw", "rub", "brl", "mxn", "sek", "nok", "dkk",
            "pln", "czk", "huf", "sgd", "hkd", "thb", "zar", "aed",
            "sar", "ils", "uah", "nzd", "btc", "eth",
            "pkr", "bdt", "lkr", "npr", "php", "idr", "myr", "vnd",
            "twd", "kwd", "qar", "omr", "bhd", "jod", "egp",
            "ngn", "kes", "ghs", "tzs",
            "ron", "bgn", "hrk", "isk",
            "cop", "ars", "clp", "pen", "uyu",
        ];
        for code in &codes {
            assert!(ct.lookup(code).is_some(),
                "ISO code '{}' should be lookupable", code);
        }
    }
}
