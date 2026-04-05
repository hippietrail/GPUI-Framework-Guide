use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_numnum() -> *const ();
}

pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_numnum) };

pub const HIGHLIGHTS_QUERY: &str = include_str!("../../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn make_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(&LANGUAGE.into())
            .expect("Failed to load numnum grammar");
        parser
    }

    fn parse(input: &str) -> tree_sitter::Tree {
        make_parser().parse(input, None).expect("Failed to parse")
    }

    fn assert_parses_without_error(input: &str) {
        let tree = parse(input);
        assert!(
            !tree.root_node().has_error(),
            "Parse error in {:?}: {}",
            input,
            tree.root_node().to_sexp()
        );
    }

    fn root_sexp(input: &str) -> String {
        parse(input).root_node().to_sexp()
    }

    // --- Basic parsing ---

    #[test]
    fn test_can_load_grammar() {
        make_parser(); // panics if grammar fails to load
    }

    #[test]
    fn test_parse_number() {
        assert_parses_without_error("42");
        assert_parses_without_error("3.14");
        assert_parses_without_error(".5");
        assert_parses_without_error("0xFF");
        assert_parses_without_error("0b1010");
        assert_parses_without_error("0o77");
        assert_parses_without_error("1,000,000");
        assert_parses_without_error("1.5e3");
    }

    #[test]
    fn test_parse_arithmetic() {
        assert_parses_without_error("2 + 3");
        assert_parses_without_error("10 - 5");
        assert_parses_without_error("4 * 5");
        assert_parses_without_error("20 / 4");
        assert_parses_without_error("2 ^ 10");
        assert_parses_without_error("17 mod 3");
    }

    #[test]
    fn test_arithmetic_structure() {
        let sexp = root_sexp("2 + 3");
        assert!(sexp.contains("binary_expression"), "expected binary_expression in: {}", sexp);
        assert!(sexp.contains("number"), "expected number nodes in: {}", sexp);
    }

    #[test]
    fn test_parse_brackets() {
        assert_parses_without_error("(2 + 3) * 4");
        assert_parses_without_error("((1 + 2) * (3 + 4))");
        assert_parses_without_error("(1 + (2 + (3 + 4)))");
    }

    #[test]
    fn test_brackets_structure() {
        let sexp = root_sexp("(2 + 3) * 4");
        assert!(sexp.contains("parenthesized"), "expected parenthesized in: {}", sexp);
        assert!(sexp.contains("binary_expression"), "expected binary_expression in: {}", sexp);
    }

    // --- Word operators ---

    #[test]
    fn test_parse_word_operators() {
        assert_parses_without_error("5 plus 3");
        assert_parses_without_error("10 minus 3");
        assert_parses_without_error("4 times 5");
        assert_parses_without_error("20 divide by 4");
        assert_parses_without_error("4 multiplied by 5");
    }

    // --- Functions ---

    #[test]
    fn test_parse_function_parens() {
        assert_parses_without_error("sqrt(16)");
        let sexp = root_sexp("sqrt(16)");
        assert!(sexp.contains("function_call"), "expected function_call in: {}", sexp);
        assert!(sexp.contains("function_name"), "expected function_name in: {}", sexp);
    }

    #[test]
    fn test_parse_function_space() {
        assert_parses_without_error("sqrt 16");
        let sexp = root_sexp("sqrt 16");
        assert!(sexp.contains("function_call"), "expected function_call in: {}", sexp);
    }

    #[test]
    fn test_parse_nested_function() {
        assert_parses_without_error("sqrt(abs(16))");
    }

    #[test]
    fn test_parse_all_functions() {
        for name in &[
            "sqrt", "cbrt", "abs", "round", "ceil", "floor",
            "log", "ln", "fact",
            "sin", "cos", "tan", "asin", "acos", "atan",
            "sinh", "cosh", "tanh",
        ] {
            let input = format!("{}(1)", name);
            assert_parses_without_error(&input);
        }
    }

    // --- Percent ---

    #[test]
    fn test_parse_percent_of() {
        assert_parses_without_error("20% of 100");
        let sexp = root_sexp("20% of 100");
        assert!(sexp.contains("percent_of"), "expected percent_of in: {}", sexp);
    }

    #[test]
    fn test_parse_percent_on_off() {
        assert_parses_without_error("5% on 100");
        assert_parses_without_error("6% off 40");
    }

    #[test]
    fn test_parse_inline_percent() {
        assert_parses_without_error("100 + 5%");
        assert_parses_without_error("100 - 5%");
    }

    #[test]
    fn test_parse_percent_literal() {
        assert_parses_without_error("50%");
        let sexp = root_sexp("50%");
        assert!(sexp.contains("percent_literal"), "expected percent_literal in: {}", sexp);
    }

    // --- Comments and headers ---

    #[test]
    fn test_parse_comment() {
        assert_parses_without_error("// this is a comment");
        let sexp = root_sexp("// a comment");
        assert!(sexp.contains("comment"), "expected comment in: {}", sexp);
    }

    #[test]
    fn test_parse_header() {
        assert_parses_without_error("# Budget");
        let sexp = root_sexp("# Budget");
        assert!(sexp.contains("header"), "expected header in: {}", sexp);
    }

    // --- Assignment ---

    #[test]
    fn test_parse_assignment() {
        assert_parses_without_error("x = 5");
        let sexp = root_sexp("x = 5");
        assert!(sexp.contains("assignment"), "expected assignment in: {}", sexp);
    }

    #[test]
    fn test_parse_compound_assignment() {
        assert_parses_without_error("x += 5");
        assert_parses_without_error("x -= 3");
        assert_parses_without_error("x *= 2");
        assert_parses_without_error("x /= 4");
    }

    // --- Currency ---

    #[test]
    fn test_parse_currency_prefix() {
        assert_parses_without_error("$10");
        let sexp = root_sexp("$10");
        assert!(sexp.contains("currency_value"), "expected currency_value in: {}", sexp);
        assert!(sexp.contains("currency_symbol"), "expected currency_symbol in: {}", sexp);
    }

    #[test]
    fn test_parse_currency_arithmetic() {
        assert_parses_without_error("$10 + $20");
    }

    // --- Scale ---

    #[test]
    fn test_parse_scaled_number() {
        assert_parses_without_error("2k");
        assert_parses_without_error("3M");
        assert_parses_without_error("5 thousand");
        assert_parses_without_error("1.5 billion");
    }

    // --- Aggregation ---

    #[test]
    fn test_parse_aggregation() {
        assert_parses_without_error("sum");
        assert_parses_without_error("total");
        assert_parses_without_error("average");
        assert_parses_without_error("avg");
        assert_parses_without_error("prev");

        let sexp = root_sexp("sum");
        assert!(sexp.contains("aggregation"), "expected aggregation in: {}", sexp);
    }

    // --- Bitwise ---

    #[test]
    fn test_parse_bitwise() {
        assert_parses_without_error("5 xor 3");
        assert_parses_without_error("1 << 8");
        assert_parses_without_error("256 >> 4");
    }

    // --- Multi-line ---

    #[test]
    fn test_parse_multiline() {
        let input = "x = 5\nx * 2\n\n10\n20\nsum";
        let tree = parse(input);
        assert!(!tree.root_node().has_error(), "multiline parse error: {}", tree.root_node().to_sexp());
    }

    // --- Error recovery ---

    #[test]
    fn test_malformed_input_has_error() {
        let tree = parse("+++");
        assert!(tree.root_node().has_error(), "expected error for +++");
    }

    #[test]
    fn test_unclosed_paren_has_error() {
        let tree = parse("(2 + 3");
        assert!(tree.root_node().has_error(), "expected error for unclosed paren");
    }

    // --- Incremental parsing ---

    #[test]
    fn test_incremental_parse() {
        let mut parser = make_parser();
        let tree1 = parser.parse("2 + 3", None).unwrap();
        assert!(!tree1.root_node().has_error());

        // Re-parse with the old tree for incrementality
        let tree2 = parser.parse("2 + 4", Some(&tree1)).unwrap();
        assert!(!tree2.root_node().has_error());
        // Result should still be a valid binary expression
        assert!(tree2.root_node().to_sexp().contains("binary_expression"));
    }
}
