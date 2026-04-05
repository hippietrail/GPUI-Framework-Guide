use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_numnum() -> *const ();
}

pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_numnum) };

pub const HIGHLIGHTS_QUERY: &str = include_str!("../../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&LANGUAGE.into())
            .expect("Failed to load numnum grammar");
    }

    #[test]
    fn test_parse_simple() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&LANGUAGE.into()).unwrap();
        let tree = parser.parse("2 + 3", None).unwrap();
        let root = tree.root_node();
        assert_eq!(root.kind(), "document");
        assert!(!root.has_error());
    }

    #[test]
    fn test_parse_function() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&LANGUAGE.into()).unwrap();
        let tree = parser.parse("sqrt(16)", None).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parse_percent() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&LANGUAGE.into()).unwrap();
        let tree = parser.parse("20% of 100", None).unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parse_comment() {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&LANGUAGE.into()).unwrap();
        let tree = parser.parse("// comment", None).unwrap();
        assert!(!tree.root_node().has_error());
    }
}
