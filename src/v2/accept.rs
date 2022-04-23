use crate::v2::negotiator::Negotiation;
use crate::v2::{parse_mime, parse_mime_with_params};
use crate::{matches, quality, Error};
use std::borrow::Cow;

pub struct AcceptNegotiation;

impl<'a> Negotiation<'a> for AcceptNegotiation {
    type ParsedSupported = (Cow<'a, str>, Cow<'a, str>, Cow<'a, str>);

    fn parse_supported<S>(mime: S) -> Result<Self::ParsedSupported, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let mime = mime.into();
        let (main, sub) = match &mime {
            Cow::Borrowed(mime) => {
                let (main, sub) = parse_mime(&mime)?;
                (Cow::from(main), Cow::from(sub))
            }
            Cow::Owned(mime) => {
                let (main, sub) = parse_mime(&mime)?;
                (Cow::from(main.to_owned()), Cow::from(sub.to_owned()))
            }
        };
        Ok((mime, main, sub))
    }

    fn negotiate<'b>(
        supported: &'b [Self::ParsedSupported],
        header: &str,
    ) -> Result<Option<&'b str>, Error> {
        let mut selected: Option<(&str, f32)> = None;
        for entry in header.split(",").map(|m| m.trim()) {
            let (_, req_main, req_sub) = parse_mime_with_params(entry)?;
            let quality = quality(entry)?;
            for (mime, main, sub) in supported {
                if let Some((_, prev_quality)) = selected {
                    if quality <= prev_quality {
                        continue;
                    }
                }
                if matches(main, req_main) && matches(sub, req_sub) {
                    selected = Some((mime, quality));
                    break;
                }
            }
        }
        Ok(selected.map(|s| s.0))
    }
}

#[cfg(test)]
mod tests {
    use crate::v2::accept::AcceptNegotiation;
    use crate::v2::negotiator::Negotiator;
    use crate::Error;
    use std::borrow::Cow;

    fn negotiate<'a, const N: usize>(
        header: &str,
        supported: [&'a str; N],
        expected: Result<Option<&str>, Error>,
    ) {
        assert_eq!(
            Negotiator::<AcceptNegotiation>::new(supported)
                .unwrap()
                .negotiate(header),
            expected
        );
    }

    #[test]
    fn negotiation() {
        // No match.
        negotiate("text/html", ["application/json"], Ok(None));
        // One to one match.
        negotiate("text/html", ["text/html"], Ok(Some("text/html")));
        // Multiple to one match.
        negotiate(
            "application/json, text/html",
            ["text/html"],
            Ok(Some("text/html")),
        );
        // One to multiple.
        negotiate(
            "text/html",
            ["application/json", "text/html"],
            Ok(Some("text/html")),
        );
        // Same quality.
        negotiate(
            "text/html, application/json",
            ["text/html", "application/json"],
            Ok(Some("text/html")),
        );
        // Subtype wildcard.
        negotiate(
            "text/*",
            ["application/json", "text/html"],
            Ok(Some("text/html")),
        );
        // Full wildcard.
        negotiate(
            "*/*",
            ["application/json", "text/html"],
            Ok(Some("application/json")),
        );
        // Weighted header.
        negotiate(
            "text/html, application/json, application/xml;q=0.9",
            ["application/xml"],
            Ok(Some("application/xml")),
        );
        // Full wildcard + weighted header.
        negotiate(
            "text/html, application/xhtml+xml, application/xml;q=0.9, */*;q=0.8",
            ["application/json"],
            Ok(Some("application/json")),
        );
        // Un-ordered weighted header.
        negotiate(
            "text/html;q=0.8, application/json;q=0.9",
            ["text/html", "application/json"],
            Ok(Some("application/json")),
        );
    }
}
