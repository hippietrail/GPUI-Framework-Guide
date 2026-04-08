use crate::types::*;
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Expr, String> {
        // Check for aggregation
        if let Some(agg) = self.try_aggregation() {
            return Ok(agg);
        }

        // Check for assignment: ident = expr  or  ident += expr
        if let Some(assign) = self.try_assignment() {
            return self.expect_eof(assign);
        }

        let expr = self.parse_expr(0)?;
        self.expect_eof(expr)
    }

    /// Ensure there are no unconsumed tokens before Eof.
    fn expect_eof(&self, expr: Expr) -> Result<Expr, String> {
        if !matches!(self.peek(), TokenKind::Eof) {
            return Err(format!("Unexpected token: {:?}", self.peek()));
        }
        Ok(expr)
    }

    fn try_aggregation(&mut self) -> Option<Expr> {
        if self.tokens.len() >= 2
            && let TokenKind::Agg(kind) = &self.tokens[0].kind
        {
            let kind = *kind;
            if matches!(self.tokens[1].kind, TokenKind::Eof) {
                self.pos = 2;
                return Some(Expr::Aggregation(kind));
            }
        }
        None
    }

    fn try_assignment(&mut self) -> Option<Expr> {
        if self.tokens.len() >= 3
            && let TokenKind::Ident(name) = &self.tokens[0].kind
        {
            let name = name.clone();
            match &self.tokens[1].kind {
                TokenKind::Assign => {
                    self.pos = 2;
                    if let Ok(value) = self.parse_expr(0) {
                        return Some(Expr::Assignment { name, value: Box::new(value) });
                    }
                    self.pos = 0;
                }
                TokenKind::CompoundAssign(op) => {
                    let op = *op;
                    self.pos = 2;
                    if let Ok(value) = self.parse_expr(0) {
                        return Some(Expr::CompoundAssignment { name, op, value: Box::new(value) });
                    }
                    self.pos = 0;
                }
                _ => {}
            }
        }
        None
    }

    fn peek(&self) -> &TokenKind {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos].kind
        } else {
            &TokenKind::Eof
        }
    }

    fn advance(&mut self) -> &Token {
        if self.pos >= self.tokens.len() {
            // Return the last token (should be Eof) without advancing past it
            return &self.tokens[self.tokens.len() - 1];
        }
        let tok = &self.tokens[self.pos];
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &TokenKind) -> Result<(), String> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.peek()))
        }
    }

    // Pratt parser core
    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_prefix()?;

        while let Some((l_bp, r_bp)) = self.infix_binding_power() {
            if l_bp < min_bp {
                break;
            }

            lhs = self.parse_infix(lhs, r_bp)?;
        }

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            TokenKind::Number(n) => {
                self.advance();
                self.maybe_attach_unit_or_currency(Expr::Number(n))
            }
            TokenKind::NumberRepr(n, repr) => {
                self.advance();
                // Check if this is a standalone literal (no operators after)
                // If followed by an operator, it'll get wrapped in BinaryOp and the repr is lost (correct: 0xFF + 1 = 256)
                // If standalone or in bitwise ops, the evaluator preserves the repr
                self.maybe_attach_unit_or_currency(Expr::NumberRepr(n, repr))
            }
            TokenKind::Op(BinOp::Sub) => {
                self.advance();
                let operand = self.parse_expr(15)?; // unary minus bp
                Ok(Expr::UnaryMinus(Box::new(operand)))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr(0)?;
                self.expect(&TokenKind::RParen)?;
                Ok(expr)
            }
            TokenKind::Func(func) => {
                self.advance();
                let arg = if matches!(self.peek(), TokenKind::LParen) {
                    self.advance();
                    let e = self.parse_expr(0)?;
                    self.expect(&TokenKind::RParen)?;
                    e
                } else {
                    self.parse_expr(17)? // function bp
                };
                Ok(Expr::FunctionCall { func, arg: Box::new(arg) })
            }
            TokenKind::CurrencySymbol(id) => {
                self.advance();
                let expr = self.parse_expr(19)?; // high bp, just grab a number
                Ok(Expr::WithCurrency { expr: Box::new(expr), currency: id })
            }
            TokenKind::Currency(id) => {
                // Currency code in prefix position: "INR 50", "USD 100"
                // Peek ahead: if next token is a number, treat as prefix currency
                if matches!(self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Number(_)) | Some(TokenKind::NumberRepr(_, _)))
                {
                    self.advance();
                    let expr = self.parse_expr(19)?;
                    Ok(Expr::WithCurrency { expr: Box::new(expr), currency: id })
                } else {
                    // Not followed by a number — error
                    self.advance();
                    Err("Unexpected currency code without number".to_string())
                }
            }
            TokenKind::Ident(ref name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Variable(name))
            }
            TokenKind::Agg(kind) => {
                self.advance();
                Ok(Expr::Aggregation(kind))
            }
            _ => Err(format!("Unexpected token: {:?}", self.peek())),
        }
    }

    fn maybe_attach_unit_or_currency(&mut self, expr: Expr) -> Result<Expr, String> {
        // Check for percent: N%
        if matches!(self.peek(), TokenKind::Percent) {
            self.advance();
            // Check for percent expressions: "of", "on", "off", "of what is", etc.
            return self.parse_percent_form(expr);
        }

        // Check for scale suffix, then continue to check for unit/currency after
        let expr = if let TokenKind::Scale(factor) = self.peek() {
            let factor = *factor;
            self.advance();
            if let Expr::Number(n) = &expr {
                Expr::Number(n * factor)
            } else {
                expr
            }
        } else {
            expr
        };

        // Check for unit, including compound form like "km/h"
        if let TokenKind::Unit(id) = self.peek() {
            let id = *id;
            self.advance();
            // Check for "/unit" compound (e.g. "km/s", "kg/L")
            if matches!(self.peek(), TokenKind::Op(BinOp::Div)) {
                let saved = self.pos;
                self.advance(); // consume /
                if let TokenKind::Unit(denom_id) = self.peek() {
                    let denom_id = *denom_id;
                    self.advance();
                    // Build: expr * (1 unit / 1 denom_unit)
                    // = BinaryOp(Div, WithUnit(expr, id), WithUnit(1, denom_id))
                    return Ok(Expr::BinaryOp {
                        op: BinOp::Div,
                        lhs: Box::new(Expr::WithUnit { expr: Box::new(expr), unit: id }),
                        rhs: Box::new(Expr::WithUnit {
                            expr: Box::new(Expr::Number(1.0)),
                            unit: denom_id,
                        }),
                    });
                }
                self.pos = saved; // backtrack if not "/ unit"
            }
            return Ok(Expr::WithUnit { expr: Box::new(expr), unit: id });
        }

        // Check for compound unit shorthand (e.g. "55 mph" → 55 mi./h)
        if let TokenKind::CompoundUnitShorthand(num_id, den_id) = self.peek() {
            let num_id = *num_id;
            let den_id = *den_id;
            self.advance();
            return Ok(Expr::BinaryOp {
                op: BinOp::Div,
                lhs: Box::new(Expr::WithUnit { expr: Box::new(expr), unit: num_id }),
                rhs: Box::new(Expr::WithUnit { expr: Box::new(Expr::Number(1.0)), unit: den_id }),
            });
        }

        // Check for currency (suffix position: "10 USD")
        if let TokenKind::Currency(id) = self.peek() {
            let id = *id;
            self.advance();
            return Ok(Expr::WithCurrency { expr: Box::new(expr), currency: id });
        }

        // Check for currency symbol suffix (e.g. "150¥", "100€", "50₹")
        if let TokenKind::CurrencySymbol(id) = self.peek() {
            let id = *id;
            self.advance();
            return Ok(Expr::WithCurrency { expr: Box::new(expr), currency: id });
        }

        Ok(expr)
    }

    fn parse_percent_form(&mut self, pct_expr: Expr) -> Result<Expr, String> {
        match self.peek() {
            TokenKind::Of => {
                self.advance();
                self.parse_percent_direction(pct_expr,
                    |pct, result| Expr::ReversePercentOf { pct, result },
                    |pct, base| Expr::PercentOf { pct, base })
            }
            TokenKind::OfWhatIs => {
                self.advance();
                let result = self.parse_expr(0)?;
                Ok(Expr::ReversePercentOf { pct: Box::new(pct_expr), result: Box::new(result) })
            }
            TokenKind::From => {
                self.advance();
                let base = self.parse_expr(0)?;
                Ok(Expr::PercentOf { pct: Box::new(pct_expr), base: Box::new(base) })
            }
            TokenKind::On => {
                self.advance();
                self.parse_percent_direction(pct_expr,
                    |pct, result| Expr::ReversePercentOn { pct, result },
                    |pct, base| Expr::PercentOn { pct, base })
            }
            TokenKind::OnWhatIs => {
                self.advance();
                let result = self.parse_expr(0)?;
                Ok(Expr::ReversePercentOn { pct: Box::new(pct_expr), result: Box::new(result) })
            }
            TokenKind::Off => {
                self.advance();
                self.parse_percent_direction(pct_expr,
                    |pct, result| Expr::ReversePercentOff { pct, result },
                    |pct, base| Expr::PercentOff { pct, base })
            }
            TokenKind::OffWhatIs => {
                self.advance();
                let result = self.parse_expr(0)?;
                Ok(Expr::ReversePercentOff { pct: Box::new(pct_expr), result: Box::new(result) })
            }
            _ => {
                // Bare percent: just N%
                Ok(Expr::Percent(Box::new(pct_expr)))
            }
        }
    }

    /// Helper for "X% of/on/off [what is] Y" patterns.
    /// If "what is" follows, builds a reverse percent expression; otherwise a forward one.
    fn parse_percent_direction(
        &mut self,
        pct_expr: Expr,
        make_reverse: fn(Box<Expr>, Box<Expr>) -> Expr,
        make_forward: fn(Box<Expr>, Box<Expr>) -> Expr,
    ) -> Result<Expr, String> {
        if matches!(self.peek(), TokenKind::Ident(w) if w.to_lowercase() == "what") {
            self.advance(); // consume "what"
            if matches!(self.peek(), TokenKind::Assign) {
                self.advance(); // consume "is"
                let result = self.parse_expr(0)?;
                return Ok(make_reverse(Box::new(pct_expr), Box::new(result)));
            }
        }
        let base = self.parse_expr(0)?;
        Ok(make_forward(Box::new(pct_expr), Box::new(base)))
    }

    fn infix_binding_power(&self) -> Option<(u8, u8)> {
        match self.peek() {
            TokenKind::Convert => Some((1, 2)),
            TokenKind::AsAPctOf | TokenKind::AsAPctOn | TokenKind::AsAPctOff => Some((3, 4)),
            TokenKind::Op(BinOp::BitAnd) | TokenKind::Op(BinOp::BitOr) | TokenKind::Op(BinOp::BitXor) => Some((5, 6)),
            TokenKind::Op(BinOp::Shl) | TokenKind::Op(BinOp::Shr) => Some((7, 8)),
            TokenKind::Op(BinOp::Add) | TokenKind::Op(BinOp::Sub) => {
                // Check for inline percent: expr + N%
                // We handle this as regular add/sub; the evaluator checks if RHS is Percent
                Some((9, 10))
            }
            TokenKind::Op(BinOp::Mul) | TokenKind::Op(BinOp::Div) | TokenKind::Op(BinOp::Mod) => Some((11, 12)),
            TokenKind::Op(BinOp::Pow) => Some((13, 14)), // LEFT-associative
            _ => None,
        }
    }

    fn parse_infix(&mut self, lhs: Expr, r_bp: u8) -> Result<Expr, String> {
        let tok = self.advance().clone();
        match &tok.kind {
            TokenKind::Op(op) => {
                let op = *op;
                let rhs = self.parse_expr(r_bp)?;

                // Check for inline percent: base +/- N%
                if (op == BinOp::Add || op == BinOp::Sub)
                    && let Expr::Percent(pct_val) = rhs
                {
                    return Ok(if op == BinOp::Add {
                        Expr::InlinePercentAdd { base: Box::new(lhs), pct: pct_val }
                    } else {
                        Expr::InlinePercentSub { base: Box::new(lhs), pct: pct_val }
                    });
                }

                Ok(Expr::BinaryOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs) })
            }
            TokenKind::Convert => {
                // Conversion target: unit, currency, or repr
                match self.peek() {
                    TokenKind::Unit(id) => {
                        let id = *id;
                        self.advance();
                        Ok(Expr::Conversion { expr: Box::new(lhs), target: ConversionTarget::Unit(id) })
                    }
                    TokenKind::Currency(id) => {
                        let id = *id;
                        self.advance();
                        Ok(Expr::Conversion { expr: Box::new(lhs), target: ConversionTarget::Currency(id) })
                    }
                    TokenKind::Repr(r) => {
                        let r = *r;
                        self.advance();
                        Ok(Expr::Conversion { expr: Box::new(lhs), target: ConversionTarget::Repr(r) })
                    }
                    TokenKind::Ident(_) => {
                        Err(format!("Unknown conversion target: {:?}", self.peek()))
                    }
                    _ => {
                        Err(format!("Expected conversion target, got {:?}", self.peek()))
                    }
                }
            }
            TokenKind::AsAPctOf => {
                let base = self.parse_expr(r_bp)?;
                Ok(Expr::AsAPercentOf { value: Box::new(lhs), base: Box::new(base) })
            }
            TokenKind::AsAPctOn => {
                let base = self.parse_expr(r_bp)?;
                Ok(Expr::AsAPercentOn { value: Box::new(lhs), base: Box::new(base) })
            }
            TokenKind::AsAPctOff => {
                let base = self.parse_expr(r_bp)?;
                Ok(Expr::AsAPercentOff { value: Box::new(lhs), base: Box::new(base) })
            }
            _ => Err(format!("Unexpected infix token: {:?}", tok.kind)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Result<Expr, String> {
        let ut = UnitTable::new();
        let ct = CurrencyTable::new();
        let mut lexer = Lexer::new(input, &ut, &ct);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_simple_add() {
        let expr = parse("2 + 3").unwrap();
        assert!(matches!(expr, Expr::BinaryOp { op: BinOp::Add, .. }));
    }

    #[test]
    fn test_precedence() {
        // 2 + 3 * 4 should be 2 + (3 * 4)
        let expr = parse("2 + 3 * 4").unwrap();
        if let Expr::BinaryOp { op: BinOp::Add, rhs, .. } = expr {
            assert!(matches!(*rhs, Expr::BinaryOp { op: BinOp::Mul, .. }));
        } else {
            panic!("Expected Add at top");
        }
    }

    #[test]
    fn test_power_left_assoc() {
        // 2 ^ 3 ^ 2 should be (2^3)^2 = 64 (left-associative)
        let expr = parse("2 ^ 3 ^ 2").unwrap();
        if let Expr::BinaryOp { op: BinOp::Pow, lhs, .. } = expr {
            assert!(matches!(*lhs, Expr::BinaryOp { op: BinOp::Pow, .. }));
        } else {
            panic!("Expected Pow at top with Pow on left");
        }
    }

    #[test]
    fn test_parens() {
        let expr = parse("(2 + 3) * 4").unwrap();
        if let Expr::BinaryOp { op: BinOp::Mul, lhs, .. } = expr {
            assert!(matches!(*lhs, Expr::BinaryOp { op: BinOp::Add, .. }));
        } else {
            panic!("Expected Mul at top");
        }
    }

    #[test]
    fn test_function() {
        let expr = parse("sqrt 16").unwrap();
        assert!(matches!(expr, Expr::FunctionCall { func: FuncKind::Sqrt, .. }));
    }

    #[test]
    fn test_function_parens() {
        let expr = parse("sqrt(16)").unwrap();
        assert!(matches!(expr, Expr::FunctionCall { func: FuncKind::Sqrt, .. }));
    }

    #[test]
    fn test_percent_of() {
        let expr = parse("20% of 100").unwrap();
        assert!(matches!(expr, Expr::PercentOf { .. }));
    }

    #[test]
    fn test_assignment() {
        let expr = parse("x = 5").unwrap();
        assert!(matches!(expr, Expr::Assignment { .. }));
    }

    #[test]
    fn test_aggregation() {
        let expr = parse("sum").unwrap();
        assert!(matches!(expr, Expr::Aggregation(AggKind::Sum)));
    }

    #[test]
    fn test_unary_minus() {
        let expr = parse("-(5)").unwrap();
        assert!(matches!(expr, Expr::UnaryMinus(_)));
    }
}
