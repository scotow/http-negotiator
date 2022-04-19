use crate::{matches, quality, Error};

pub trait Negotiator<'a, 'b> {
    type SupportedIter;

    fn supported(&'b self) -> Self::SupportedIter;

    fn negotiate(&'b self, header: &str) -> Result<Option<&'a str>, Error>
    where
        'a: 'b,
        Self::SupportedIter: Iterator<Item = (&'a str, &'a str, &'a str)>,
    {
        let mut selected: Option<(&str, f32)> = None;
        for entry in header.split(",").map(|m| m.trim()) {
            let req_full = entry.split(';').next().ok_or(Error::InvalidHeader)?;
            let (req_main, req_sub) = req_full.split_once('/').ok_or(Error::MissingSeparator)?;
            if req_main == "*" && req_sub != "*" {
                return Err(Error::InvalidWildcard);
            }
            let quality = quality(entry)?;

            for (full, main, sub) in self.supported() {
                if let Some((_, prev_quality)) = selected {
                    if quality < prev_quality {
                        continue;
                    }
                }
                if matches(main, req_main) && matches(sub, req_sub) {
                    selected = Some((full, quality));
                    break;
                }
            }
        }
        Ok(selected.map(|s| s.0))
    }
}

#[cfg(test)]
mod tests {
    use super::Error;
    use super::Negotiator;
    use crate::accept::negotiator_ref::NegotiatorRef;

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
