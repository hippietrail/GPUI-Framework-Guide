use std::ops::Range;
use crate::types::*;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(f64),
    NumberRepr(f64, NumRepr),
    Op(BinOp),
    LParen, RParen,
    Percent,
    Assign,
    CompoundAssign(CompoundOp),
    Convert, // in, to, as, into
    Of, From, On, Off,
    AsAPctOf, AsAPctOn, AsAPctOff,
    OfWhatIs, OnWhatIs, OffWhatIs,
    Func(FuncKind),
    Unit(UnitId),
    Currency(CurrencyId),
    CurrencySymbol(CurrencyId),
    Scale(f64),
    Repr(ReprKind),
    Ident(String),
    Agg(AggKind),
    Comment,
    Header,
    Label(String),
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Range<usize>,
}

pub struct Lexer<'a> {
    /// The owned, processed input (quotes stripped). We store it here
    /// so the Lexer owns the data instead of leaking it.
    processed_input: String,
    pos: usize,
    unit_table: &'a UnitTable,
    currency_table: &'a CurrencyTable,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &str, unit_table: &'a UnitTable, currency_table: &'a CurrencyTable) -> Self {
        let processed_input = strip_quotes(input);
        Lexer { processed_input, pos: 0, unit_table, currency_table }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        // Pre-allocate: a rough estimate is ~1 token per 3 chars of input
        let mut tokens = Vec::with_capacity(self.processed_input.len() / 3 + 1);
        self.pos = 0;

        // Check for comment/header
        let trimmed = self.processed_input.trim_start();
        if trimmed.starts_with("//") {
            tokens.push(Token { kind: TokenKind::Comment, span: 0..self.processed_input.len() });
            return tokens;
        }
        if trimmed.starts_with('#') {
            tokens.push(Token { kind: TokenKind::Header, span: 0..self.processed_input.len() });
            return tokens;
        }

        // Check for label: "word: rest"
        if let Some(colon_pos) = self.processed_input.find(':') {
            let before = &self.processed_input[..colon_pos];
            if !before.is_empty() && before.chars().all(|c| c.is_alphanumeric() || c == '_')
                && colon_pos + 1 < self.processed_input.len()
            {
                let label = before.to_string();
                tokens.push(Token { kind: TokenKind::Label(label), span: 0..colon_pos + 1 });
                self.pos = colon_pos + 1;
                self.skip_whitespace();
            }
        }

        loop {
            self.skip_whitespace();
            if self.pos >= self.processed_input.len() {
                break;
            }
            if let Some(tok) = self.next_token() {
                tokens.push(tok);
            } else {
                // Advance past the unknown character by its full UTF-8 width,
                // not just 1 byte, to avoid landing inside a multi-byte char.
                if let Some(c) = self.peek_char() {
                    self.pos += c.len_utf8();
                } else {
                    self.pos += 1;
                }
            }
        }
        tokens.push(Token { kind: TokenKind::Eof, span: self.pos..self.pos });
        tokens
    }

    fn input(&self) -> &str {
        &self.processed_input
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input().len() {
            let c = self.input().as_bytes()[self.pos];
            if c == b' ' || c == b'\t' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn remaining(&self) -> &str {
        &self.input()[self.pos..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn next_token(&mut self) -> Option<Token> {
        let start = self.pos;
        let c = self.peek_char()?;

        // Currency symbols (prefix)
        if let Some(tok) = self.try_currency_symbol(start) {
            return Some(tok);
        }

        // Two-char operators (only ASCII pairs, so check that both bytes are ASCII first)
        if self.pos + 1 < self.input().len()
            && self.input().as_bytes()[self.pos].is_ascii()
            && self.input().as_bytes()[self.pos + 1].is_ascii()
        {
            let two = &self.processed_input[self.pos..self.pos + 2];
            let kind = match two {
                "<<" => Some(TokenKind::Op(BinOp::Shl)),
                ">>" => Some(TokenKind::Op(BinOp::Shr)),
                "+=" => Some(TokenKind::CompoundAssign(CompoundOp::AddAssign)),
                "-=" => Some(TokenKind::CompoundAssign(CompoundOp::SubAssign)),
                "*=" => Some(TokenKind::CompoundAssign(CompoundOp::MulAssign)),
                "/=" => Some(TokenKind::CompoundAssign(CompoundOp::DivAssign)),
                _ => None,
            };
            if let Some(kind) = kind {
                self.pos += 2;
                return Some(Token { kind, span: start..self.pos });
            }
        }

        // Single-char operators
        let kind = match c {
            '+' => Some(TokenKind::Op(BinOp::Add)),
            '-' => Some(TokenKind::Op(BinOp::Sub)),
            '*' => Some(TokenKind::Op(BinOp::Mul)),
            '/' => Some(TokenKind::Op(BinOp::Div)),
            '^' => Some(TokenKind::Op(BinOp::Pow)),
            '%' => Some(TokenKind::Percent),
            '&' => Some(TokenKind::Op(BinOp::BitAnd)),
            '|' => Some(TokenKind::Op(BinOp::BitOr)),
            '(' => Some(TokenKind::LParen),
            ')' => Some(TokenKind::RParen),
            '=' => Some(TokenKind::Assign),
            '\u{00D7}' => Some(TokenKind::Op(BinOp::Mul)), // x
            '\u{00F7}' => Some(TokenKind::Op(BinOp::Div)), // div
            _ => None,
        };
        if let Some(kind) = kind {
            self.pos += c.len_utf8();
            return Some(Token { kind, span: start..self.pos });
        }

        // Numbers
        if c.is_ascii_digit() || (c == '.' && self.remaining().len() > 1 && self.remaining().as_bytes()[1].is_ascii_digit()) {
            return Some(self.lex_number(start));
        }

        // Words (identifiers, keywords, word operators, functions, units, currencies)
        if c.is_alphabetic() || c == '_' {
            return Some(self.lex_word(start));
        }

        None
    }

    fn try_currency_symbol(&mut self, start: usize) -> Option<Token> {
        let c = self.peek_char()?;
        let (symbol, currency_code) = match c {
            '$' => ("$", "USD"),
            '\u{20AC}' => ("\u{20AC}", "EUR"),   // €
            '\u{00A3}' => ("\u{00A3}", "GBP"),   // £
            '\u{00A5}' => ("\u{00A5}", "JPY"),   // ¥
            '\u{20BD}' => ("\u{20BD}", "RUB"),   // ₽
            '\u{20AA}' => ("\u{20AA}", "ILS"),   // ₪
            '\u{20B9}' => ("\u{20B9}", "INR"),   // ₹
            '\u{20A9}' => ("\u{20A9}", "KRW"),   // ₩
            '\u{20B4}' => ("\u{20B4}", "UAH"),   // ₴
            '\u{20BF}' => ("\u{20BF}", "BTC"),   // ₿
            '\u{20BA}' => ("\u{20BA}", "TRY"),   // ₺
            '\u{0E3F}' => ("\u{0E3F}", "THB"),   // ฿
            '\u{20B1}' => ("\u{20B1}", "PHP"),   // ₱
            '\u{20A6}' => ("\u{20A6}", "NGN"),   // ₦
            '\u{20AB}' => ("\u{20AB}", "VND"),   // ₫
            '\u{20A8}' => ("\u{20A8}", "PKR"),   // ₨
            '\u{09F3}' => ("\u{09F3}", "BDT"),   // ৳
            '\u{20B8}' => ("\u{20B8}", "KZT"),   // ₸
            '\u{20BC}' => ("\u{20BC}", "AZN"),   // ₼
            '\u{20BE}' => ("\u{20BE}", "GEL"),   // ₾
            '\u{058F}' => ("\u{058F}", "AMD"),   // ֏
            '\u{17DB}' => ("\u{17DB}", "KHR"),   // ៛
            '\u{20AD}' => ("\u{20AD}", "LAK"),   // ₭
            '\u{20AE}' => ("\u{20AE}", "MNT"),   // ₮
            '\u{20A1}' => ("\u{20A1}", "CRC"),   // ₡
            '\u{20B2}' => ("\u{20B2}", "PYG"),   // ₲
            '\u{0192}' => ("\u{0192}", "ANG"),   // ƒ
            _ => return None,
        };
        if let Some(id) = self.currency_table.lookup(currency_code) {
            self.pos += symbol.len();
            Some(Token { kind: TokenKind::CurrencySymbol(id), span: start..self.pos })
        } else {
            None
        }
    }

    fn lex_number(&mut self, start: usize) -> Token {
        let input_len = self.input().len();
        // Hex
        if self.remaining().starts_with("0x") || self.remaining().starts_with("0X") {
            self.pos += 2;
            while self.pos < input_len && self.input().as_bytes()[self.pos].is_ascii_hexdigit() {
                self.pos += 1;
            }
            let hex_str = &self.processed_input[start + 2..self.pos];
            let val = u64::from_str_radix(hex_str, 16).unwrap_or(0) as f64;
            return Token { kind: TokenKind::NumberRepr(val, NumRepr::Hex), span: start..self.pos };
        }
        // Binary
        if self.remaining().starts_with("0b") {
            self.pos += 2;
            while self.pos < input_len && (self.input().as_bytes()[self.pos] == b'0' || self.input().as_bytes()[self.pos] == b'1') {
                self.pos += 1;
            }
            let bin_str = &self.processed_input[start + 2..self.pos];
            let val = u64::from_str_radix(bin_str, 2).unwrap_or(0) as f64;
            return Token { kind: TokenKind::NumberRepr(val, NumRepr::Binary), span: start..self.pos };
        }
        // Octal
        if self.remaining().starts_with("0o") || self.remaining().starts_with("0O") {
            self.pos += 2;
            while self.pos < input_len && self.input().as_bytes()[self.pos] >= b'0' && self.input().as_bytes()[self.pos] <= b'7' {
                self.pos += 1;
            }
            let oct_str = &self.processed_input[start + 2..self.pos];
            let val = u64::from_str_radix(oct_str, 8).unwrap_or(0) as f64;
            return Token { kind: TokenKind::NumberRepr(val, NumRepr::Octal), span: start..self.pos };
        }

        // Decimal (with comma separators)
        while self.pos < input_len {
            let b = self.input().as_bytes()[self.pos];
            if b.is_ascii_digit() || b == b',' || b == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        // Scientific notation
        if self.pos < input_len && (self.input().as_bytes()[self.pos] == b'e' || self.input().as_bytes()[self.pos] == b'E') {
            self.pos += 1;
            if self.pos < input_len && (self.input().as_bytes()[self.pos] == b'+' || self.input().as_bytes()[self.pos] == b'-') {
                self.pos += 1;
            }
            while self.pos < input_len && self.input().as_bytes()[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        let num_str: String = self.processed_input[start..self.pos].chars().filter(|c| *c != ',').collect();
        let val = num_str.parse::<f64>().unwrap_or(0.0);

        // Check for immediate scale suffix (no space): 2k, 3M
        if self.pos < input_len {
            let next = self.input().as_bytes()[self.pos];
            if next == b'k' && !self.is_word_char_at(self.pos + 1) {
                self.pos += 1;
                return Token { kind: TokenKind::Number(val * 1e3), span: start..self.pos };
            }
            // M is ambiguous -- treated as million scale when immediately after number
            if next == b'M' && !self.is_word_char_at(self.pos + 1) {
                self.pos += 1;
                return Token { kind: TokenKind::Number(val * 1e6), span: start..self.pos };
            }
        }

        Token { kind: TokenKind::Number(val), span: start..self.pos }
    }

    fn is_word_char_at(&self, pos: usize) -> bool {
        if pos >= self.input().len() { return false; }
        if !self.input().is_char_boundary(pos) { return false; }
        match self.input()[pos..].chars().next() {
            Some(c) => c.is_alphanumeric() || c == '_',
            None => false,
        }
    }

    fn lex_word(&mut self, start: usize) -> Token {
        // Consume the full word (supports Unicode alphabetic characters)
        while self.pos < self.input().len() {
            match self.input()[self.pos..].chars().next() {
                Some(c) if c.is_alphanumeric() || c == '_' || c == '.' => {
                    self.pos += c.len_utf8();
                }
                _ => break,
            }
        }
        let word = self.processed_input[start..self.pos].to_string();
        let lower = word.to_lowercase();

        // If word is followed by `=` or compound-assign, treat as identifier
        // so that assignments like `em = 14` work even if "em" is a unit name
        if self.peek_is_assignment() {
            return Token { kind: TokenKind::Ident(word), span: start..self.pos };
        }

        // Try multi-word tokens (look ahead)
        if let Some(tok) = self.try_multi_word(&lower, start) {
            return tok;
        }

        // Try word operator
        if let Some(tok) = self.try_word_operator(&lower, start) {
            return tok;
        }

        // Try conversion keyword
        if let Some(tok) = self.try_conversion_keyword(&lower, start) {
            return tok;
        }

        // Try percent keyword
        if let Some(tok) = self.try_percent_keyword(&lower, start) {
            return tok;
        }

        // Try function
        if let Some(tok) = self.try_function(&lower, start) {
            return tok;
        }

        // Try constant
        if let Some(tok) = self.try_constant(&word, start) {
            return tok;
        }

        // Try aggregation
        if let Some(tok) = self.try_aggregation(&lower, start) {
            return tok;
        }

        // Try repr keyword
        if let Some(tok) = self.try_repr_keyword(&lower, start) {
            return tok;
        }

        // Try scale word
        if let Some(tok) = self.try_scale_word(&lower, start) {
            return tok;
        }

        // Assignment keywords
        if lower == "equal" || lower == "is" {
            return Token { kind: TokenKind::Assign, span: start..self.pos };
        }

        // Unit lookup
        if let Some(id) = self.unit_table.lookup(&lower) {
            return Token { kind: TokenKind::Unit(id), span: start..self.pos };
        }

        // Currency lookup (try compound symbol with trailing '$' first: R$, HK$, S$)
        if self.pos < self.input().len() && self.peek_char() == Some('$') {
            let compound = format!("{}$", lower);
            if let Some(id) = self.currency_table.lookup(&compound) {
                self.pos += 1; // consume the '$'
                return Token { kind: TokenKind::CurrencySymbol(id), span: start..self.pos };
            }
        }
        if let Some(id) = self.currency_table.lookup(&lower) {
            return Token { kind: TokenKind::Currency(id), span: start..self.pos };
        }

        // Plain identifier (variable)
        Token { kind: TokenKind::Ident(word), span: start..self.pos }
    }

    fn try_word_operator(&self, lower: &str, start: usize) -> Option<Token> {
        let op = match lower {
            "plus" | "with" | "and" => BinOp::Add,
            "minus" | "without" | "subtract" => BinOp::Sub,
            "times" | "mul" | "mult" | "multiply" => BinOp::Mul,
            "divide" => BinOp::Div,
            "mod" => BinOp::Mod,
            "xor" => BinOp::BitXor,
            _ => return None,
        };
        Some(Token { kind: TokenKind::Op(op), span: start..self.pos })
    }

    fn try_conversion_keyword(&self, lower: &str, start: usize) -> Option<Token> {
        match lower {
            "in" | "to" | "into" | "as" => {
                Some(Token { kind: TokenKind::Convert, span: start..self.pos })
            }
            _ => None,
        }
    }

    fn try_percent_keyword(&self, lower: &str, start: usize) -> Option<Token> {
        match lower {
            "of" => Some(Token { kind: TokenKind::Of, span: start..self.pos }),
            "from" => Some(Token { kind: TokenKind::From, span: start..self.pos }),
            "on" => Some(Token { kind: TokenKind::On, span: start..self.pos }),
            "off" => Some(Token { kind: TokenKind::Off, span: start..self.pos }),
            "percent" | "percents" | "pct" | "pct." => {
                Some(Token { kind: TokenKind::Percent, span: start..self.pos })
            }
            _ => None,
        }
    }

    fn try_function(&self, lower: &str, start: usize) -> Option<Token> {
        let func = match lower {
            "sqrt" => FuncKind::Sqrt,
            "cbrt" => FuncKind::Cbrt,
            "abs" => FuncKind::Abs,
            "round" => FuncKind::Round,
            "ceil" => FuncKind::Ceil,
            "floor" => FuncKind::Floor,
            "log" => FuncKind::Log,
            "ln" => FuncKind::Ln,
            "fact" => FuncKind::Fact,
            "sin" => FuncKind::Sin,
            "cos" => FuncKind::Cos,
            "tan" => FuncKind::Tan,
            "asin" | "arcsin" => FuncKind::Asin,
            "acos" | "arccos" => FuncKind::Acos,
            "atan" | "arctan" => FuncKind::Atan,
            "sinh" => FuncKind::Sinh,
            "cosh" => FuncKind::Cosh,
            "tanh" => FuncKind::Tanh,
            _ => return None,
        };
        Some(Token { kind: TokenKind::Func(func), span: start..self.pos })
    }

    fn try_constant(&self, word: &str, start: usize) -> Option<Token> {
        let val = match word {
            "pi" | "Pi" | "PI" => std::f64::consts::PI,
            "e" | "E" => std::f64::consts::E,
            _ => return None,
        };
        Some(Token { kind: TokenKind::Number(val), span: start..self.pos })
    }

    fn try_aggregation(&self, lower: &str, start: usize) -> Option<Token> {
        let kind = match lower {
            "sum" | "total" => AggKind::Sum,
            "average" | "avg" => AggKind::Average,
            "prev" => AggKind::Prev,
            _ => return None,
        };
        Some(Token { kind: TokenKind::Agg(kind), span: start..self.pos })
    }

    fn try_repr_keyword(&self, lower: &str, start: usize) -> Option<Token> {
        let repr = match lower {
            "hex" => ReprKind::Hex,
            "binary" | "bin" => ReprKind::Binary,
            "octal" | "oct" => ReprKind::Octal,
            "decimal" | "dec" => ReprKind::Decimal,
            "scientific" | "sci" | "exp" | "exponent" | "exponential" => ReprKind::Scientific,
            _ => return None,
        };
        Some(Token { kind: TokenKind::Repr(repr), span: start..self.pos })
    }

    fn try_scale_word(&self, lower: &str, start: usize) -> Option<Token> {
        let scale = match lower {
            "thousand" | "thousands" | "th" | "th." => 1e3,
            "million" | "millions" => 1e6,
            "billion" | "billions" | "milliard" | "milliards" => 1e9,
            "trillion" | "trillions" => 1e12,
            "quadrillion" | "quadrillions" => 1e15,
            "quintillion" | "quintillions" => 1e18,
            "sextillion" | "sextillions" => 1e21,
            "septillion" | "septillions" => 1e24,
            _ => return None,
        };
        Some(Token { kind: TokenKind::Scale(scale), span: start..self.pos })
    }

    fn try_multi_word(&mut self, first_word: &str, start: usize) -> Option<Token> {
        let saved_pos = self.pos;

        match first_word {
            "as" => {
                // Try "as a % of" / "as a % on" / "as a % off"
                if self.try_consume_word("a") && self.try_consume_char('%') {
                    let after_pct = self.pos;
                    if self.try_consume_word("of") {
                        return Some(Token { kind: TokenKind::AsAPctOf, span: start..self.pos });
                    }
                    self.pos = after_pct;
                    if self.try_consume_word("on") {
                        return Some(Token { kind: TokenKind::AsAPctOn, span: start..self.pos });
                    }
                    self.pos = after_pct;
                    if self.try_consume_word("off") {
                        return Some(Token { kind: TokenKind::AsAPctOff, span: start..self.pos });
                    }
                }
            }
            "divided" | "divide" => {
                if self.try_consume_word("by") {
                    return Some(Token { kind: TokenKind::Op(BinOp::Div), span: start..self.pos });
                }
            }
            "multiplied" => {
                if self.try_consume_word("by") {
                    return Some(Token { kind: TokenKind::Op(BinOp::Mul), span: start..self.pos });
                }
            }
            "square" => {
                if self.try_consume_word("root") {
                    return Some(Token { kind: TokenKind::Func(FuncKind::Sqrt), span: start..self.pos });
                }
            }
            "cube" | "cubic" | "cubed" => {
                if self.try_consume_word("root") {
                    return Some(Token { kind: TokenKind::Func(FuncKind::Cbrt), span: start..self.pos });
                }
            }
            "nautical" => {
                if (self.try_consume_word("mile") || self.try_consume_word("miles"))
                    && let Some(id) = self.unit_table.lookup("nautical mile")
                {
                    return Some(Token { kind: TokenKind::Unit(id), span: start..self.pos });
                }
            }
            _ => {}
        }

        self.pos = saved_pos;
        None
    }

    /// Check if the next non-whitespace content is `=`, `+=`, `-=`, `*=`, `/=`
    /// (i.e., the word we just lexed is an assignment target).
    fn peek_is_assignment(&self) -> bool {
        let mut p = self.pos;
        let bytes = self.input().as_bytes();
        while p < bytes.len() && (bytes[p] == b' ' || bytes[p] == b'\t') {
            p += 1;
        }
        if p >= bytes.len() { return false; }
        if bytes[p] == b'=' {
            // Make sure it's not `==` (which we don't have, but guard anyway)
            return p + 1 >= bytes.len() || bytes[p + 1] != b'=';
        }
        // Check compound assignment: +=, -=, *=, /=
        if p + 1 < bytes.len() && bytes[p + 1] == b'=' {
            return matches!(bytes[p], b'+' | b'-' | b'*' | b'/');
        }
        false
    }

    fn try_consume_char(&mut self, expected: char) -> bool {
        let saved = self.pos;
        self.skip_whitespace();
        if self.pos < self.input().len() {
            let c = self.remaining().chars().next();
            if c == Some(expected) {
                self.pos += expected.len_utf8();
                return true;
            }
        }
        self.pos = saved;
        false
    }

    fn try_consume_word(&mut self, expected: &str) -> bool {
        let saved = self.pos;
        self.skip_whitespace();
        let word_start = self.pos;
        while self.pos < self.input().len() {
            match self.input()[self.pos..].chars().next() {
                Some(c) if c.is_alphanumeric() => {
                    self.pos += c.len_utf8();
                }
                _ => break,
            }
        }
        let word = &self.processed_input[word_start..self.pos];
        if word.eq_ignore_ascii_case(expected) {
            true
        } else {
            self.pos = saved;
            false
        }
    }
}

fn strip_quotes(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_quote = false;
    for c in input.chars() {
        if c == '"' {
            in_quote = !in_quote;
        } else if !in_quote {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tables() -> (UnitTable, CurrencyTable) {
        (UnitTable::new(), CurrencyTable::new())
    }

    #[test]
    fn test_basic_number() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("42", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 42.0).abs() < 1e-10));
    }

    #[test]
    fn test_arithmetic() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("2 + 3 * 4", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Number(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Op(BinOp::Add)));
        assert!(matches!(tokens[2].kind, TokenKind::Number(_)));
        assert!(matches!(tokens[3].kind, TokenKind::Op(BinOp::Mul)));
        assert!(matches!(tokens[4].kind, TokenKind::Number(_)));
    }

    #[test]
    fn test_hex() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("0xFF", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::NumberRepr(n, NumRepr::Hex) if (n - 255.0).abs() < 1e-10));
    }

    #[test]
    fn test_word_operators() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("5 plus 3", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[1].kind, TokenKind::Op(BinOp::Add)));
    }

    #[test]
    fn test_comment() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("// this is a comment", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Comment));
    }

    #[test]
    fn test_header() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("# Budget", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Header));
    }

    #[test]
    fn test_scale() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("2k", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 2000.0).abs() < 1e-10));
    }

    #[test]
    fn test_currency_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("$10", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if (n - 10.0).abs() < 1e-10));
    }

    #[test]
    fn test_label() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("Price: 10", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(&tokens[0].kind, TokenKind::Label(l) if l == "Price"));
    }

    #[test]
    fn test_function() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("sqrt 16", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Func(FuncKind::Sqrt)));
    }

    #[test]
    fn test_quoted_text_stripped() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("100 + 50 \"tax\"", &ut, &ct);
        let tokens = l.tokenize();
        // Should have: 100, +, 50, Eof -- "tax" stripped
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 100.0).abs() < 1e-10));
        assert!(matches!(tokens[1].kind, TokenKind::Op(BinOp::Add)));
        assert!(matches!(tokens[2].kind, TokenKind::Number(n) if (n - 50.0).abs() < 1e-10));
    }

    #[test]
    fn test_comma_number() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("1,000,000", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::Number(n) if (n - 1_000_000.0).abs() < 1e-10));
    }

    #[test]
    fn test_inr_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("\u{20B9}500", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
        assert!(matches!(tokens[1].kind, TokenKind::Number(n) if (n - 500.0).abs() < 1e-10));
    }

    #[test]
    fn test_php_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("\u{20B1}100", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
    }

    #[test]
    fn test_ngn_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("\u{20A6}5000", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
    }

    #[test]
    fn test_vnd_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("\u{20AB}10000", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
    }

    #[test]
    fn test_pkr_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("\u{20A8}1000", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
    }

    #[test]
    fn test_bdt_symbol() {
        let (ut, ct) = make_tables();
        let mut l = Lexer::new("\u{09F3}500", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)));
    }

    #[test]
    fn test_new_currency_words() {
        let (ut, ct) = make_tables();
        // Test that word-based currency names tokenize as Currency
        let test_cases = [
            ("100 PKR", "PKR"),
            ("100 PHP", "PHP"),
            ("100 IDR", "IDR"),
            ("100 MYR", "MYR"),
            ("100 VND", "VND"),
            ("100 TWD", "TWD"),
            ("100 EGP", "EGP"),
            ("100 NGN", "NGN"),
        ];
        for (input, _code) in &test_cases {
            let mut l = Lexer::new(input, &ut, &ct);
            let tokens = l.tokenize();
            assert!(matches!(tokens[0].kind, TokenKind::Number(_)),
                "Expected number for input '{}'", input);
            assert!(matches!(tokens[1].kind, TokenKind::Currency(_)),
                "Expected currency token for input '{}'", input);
        }
    }

    #[test]
    fn test_variant_words() {
        let (ut, ct) = make_tables();
        // Test common variant words tokenize as Currency
        let variants = [
            "rmb", "renminbi", "quid", "bucks", "rupees",
        ];
        for v in &variants {
            let input = format!("100 {}", v);
            let mut l = Lexer::new(&input, &ut, &ct);
            let tokens = l.tokenize();
            assert!(matches!(tokens[1].kind, TokenKind::Currency(_)),
                "Variant '{}' should tokenize as Currency", v);
        }
    }

    #[test]
    fn test_compound_symbol_nt_dollar() {
        let (ut, ct) = make_tables();
        // NT$ should be recognized as a compound currency symbol
        let mut l = Lexer::new("NT$100", &ut, &ct);
        let tokens = l.tokenize();
        assert!(matches!(tokens[0].kind, TokenKind::CurrencySymbol(_)),
            "NT$ should tokenize as CurrencySymbol");
    }
}
