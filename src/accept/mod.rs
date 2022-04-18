pub mod negotiator;
pub mod negotiator_owned;
pub mod negotiator_ref;

#[cfg(test)]
mod tests {
    use super::negotiator_ref::NegotiatorRef;
    use crate::accept::negotiator::Negotiator;
    use crate::Error;

    fn negotiate<'a, const N: usize>(
        header: &str,
        supported: [&'a str; N],
    ) -> Result<Option<&'a str>, Error> {
        NegotiatorRef::new(supported).unwrap().negotiate(header)
    }

    #[test]
    fn negotiation() {
        // No match.
        assert_eq!(negotiate("text/html", ["application/json"]), Ok(None));
        // One to one match.
        assert_eq!(negotiate("text/html", ["text/html"]), Ok(Some("text/html")));
        // Multiple to one match.
        assert_eq!(
            negotiate("application/json, text/html", ["text/html"]),
            Ok(Some("text/html"))
        );
        // One to multiple.
        assert_eq!(
            negotiate("text/html", ["application/json", "text/html"]),
            Ok(Some("text/html"))
        );
        // Subtype wildcard.
        assert_eq!(
            negotiate("text/*", ["application/json", "text/html"]),
            Ok(Some("text/html"))
        );
        // Full wildcard.
        assert_eq!(
            negotiate("*/*", ["application/json", "text/html"]),
            Ok(Some("application/json"))
        );
        // Weighted header.
        assert_eq!(
            negotiate(
                "text/html, application/json, application/xml;q=0.9",
                ["application/xml"]
            ),
            Ok(Some("application/xml"))
        );
        // Full wildcard + weighted header.
        assert_eq!(
            negotiate(
                "text/html, application/xhtml+xml, application/xml;q=0.9, */*;q=0.8",
                ["application/json"]
            ),
            Ok(Some("application/json")),
        );
        // Un-ordered weighted header.
        assert_eq!(
            negotiate(
                "text/html;q=0.8, application/json;q=0.9",
                ["text/html", "application/json"]
            ),
            Ok(Some("application/json")),
        );
    }
}
