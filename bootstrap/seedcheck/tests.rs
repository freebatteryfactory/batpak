    use crate::tokens::{is_test_owned, sanitize_rust};
    use std::path::Path;

    fn production_debt(code: &str) -> Vec<String> {
        let sanitized = sanitize_rust(code);
        let mut hits = Vec::new();
        for token in [".unwrap(", ".expect(", "panic!", "dbg!"] {
            if sanitized.contains(token) { hits.push(token.to_string()); }
        }
        hits
    }

    #[test]
    fn production_panic_is_rejected() {
        assert!(production_debt("fn f() { panic!(\"boom\"); }").iter().any(|h| h == "panic!"));
    }

    #[test]
    fn production_unwrap_is_rejected() {
        assert!(production_debt("fn f() { let _ = x.unwrap(); }").iter().any(|h| h == ".unwrap("));
    }

    #[test]
    fn production_expect_is_rejected() {
        assert!(production_debt("fn f() { let _ = x.expect(\"why\"); }").iter().any(|h| h == ".expect("));
    }

    #[test]
    fn production_dbg_is_rejected() {
        assert!(production_debt("fn f() { dbg!(x); }").iter().any(|h| h == "dbg!"));
    }

    #[test]
    fn commented_and_string_tokens_are_ignored() {
        assert!(production_debt("// panic!\nlet s = \".unwrap(\";").is_empty());
    }

    #[test]
    fn test_path_expect_is_allowed() {
        assert!(is_test_owned(Path::new("crates/batpak/tests/recovery.rs")));
        assert!(is_test_owned(Path::new("crates/testpak/fixtures/x.rs")));
        assert!(!is_test_owned(Path::new("crates/batpak/src/event.rs")));
    }

    #[test]
    fn bootstrap_detector_fixture_does_not_grade_itself() {
        assert!(production_debt("let banned = [r#\"panic!\"#, r\".unwrap(\"];").is_empty());
    }

    #[test]
    fn module_declarations_parse_visibility_and_attributes() {
        let decls = crate::grammar::module_declarations(
            "mod types;\npub mod door;\n#[cfg(test)] mod tests;\npub(crate) mod inner;\n");
        assert_eq!(decls, vec![
            ("types".to_string(), false),
            ("door".to_string(), true),
            ("tests".to_string(), false),
            ("inner".to_string(), true),
        ]);
    }

    #[test]
    fn glob_reexport_is_detected_and_explicit_lists_are_not() {
        assert!(crate::grammar::has_glob_reexport("pub use types::*;"));
        assert!(crate::grammar::has_glob_reexport("pub use types::{A, inner::*};"));
        assert!(!crate::grammar::has_glob_reexport("pub use types::{A, B};\nuse super::*;"));
    }
