//! The canonical form of an email: the login identifier.

/// The canonical form of `email`: surrounding whitespace removed and lower case.
///
/// Whitespace is what [`str::trim`] treats as such. The `users_email_canonical` constraint on
/// `identity.users` (migration `m20261019_000001_require_canonical_email_identity`) lists the
/// same characters; `tests/schema.rs` checks that the two agree.
pub(crate) fn normalize(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn removes_surrounding_spaces() {
        assert_eq!(normalize("  alex@example.edu  "), "alex@example.edu");
    }

    #[test]
    fn removes_surrounding_tabs_and_line_breaks() {
        assert_eq!(normalize("\t\nalex@example.edu\r\n"), "alex@example.edu");
    }

    #[test]
    fn removes_unicode_whitespace() {
        assert_eq!(
            normalize("\u{a0}alex@example.edu\u{3000}"),
            "alex@example.edu"
        );
    }

    #[test]
    fn lower_cases() {
        assert_eq!(normalize("ALEX@Example.EDU"), "alex@example.edu");
    }

    #[test]
    fn leaves_text_between_the_ends_alone() {
        assert_eq!(normalize("a b@example.edu"), "a b@example.edu");
    }

    #[test]
    fn is_idempotent() {
        let once = normalize(" Alex@Example.edu ");
        assert_eq!(normalize(&once), once);
    }
}
