mod accept;
mod error;

use std::{borrow::Borrow, collections::BTreeMap};

pub use accept::Negotiator as AcceptNegotiator;
pub use error::Error;

fn parse_mime<'a, T>(mime: &'a str, from_header: bool) -> Result<(T, T, BTreeMap<T, T>), Error>
where
    T: From<&'a str> + Ord + Borrow<str>,
{
    let mut parts = mime.split(';');
    let left = parts.next().ok_or(Error::InvalidHeader)?.trim();

    let (main, sub) = left.split_once('/').ok_or(Error::MissingSeparator)?;
    if sub.contains('/') {
        return Err(Error::TooManyPart);
    }
    if from_header {
        if main == "*" && sub != "*" {
            return Err(Error::InvalidWildcard);
        }
    } else {
        if main == "*" || sub == "*" {
            return Err(Error::InvalidWildcard);
        }
    }

    let params = parts
        .map(|param| {
            let (k, v) = param.trim().split_once('=').ok_or(Error::InvalidHeader)?;
            Ok((k.into(), v.into()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    if !from_header && params.contains_key("q") {
        return Err(Error::QualityNotAllowed);
    }

    Ok((main.into(), sub.into(), params))
}

fn extract_quality(params: &mut BTreeMap<&str, &str>) -> Result<f32, Error> {
    params
        .remove("q")
        .map(|q| {
            q.parse::<f32>()
                .map_err(|err| Error::InvalidQuality { source: err })
        })
        .transpose()
        .map(|q| q.unwrap_or(1.))
}

fn mime_score(main: &str, sub: &str) -> u8 {
    match (main, sub) {
        ("*", "*") => 0,
        (_, "*") => 1,
        _ => 2,
    }
}

fn matches(specific: &str, maybe_wildcard: &str) -> bool {
    specific == maybe_wildcard || maybe_wildcard == "*"
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::parse_mime;
    use crate::Error;

    #[test]
    fn parse() {
        // Basic.
        assert_eq!(
            parse_mime("text/plain", false).unwrap(),
            ("text", "plain", BTreeMap::default()),
        );

        // With one param.
        assert_eq!(
            parse_mime("text/html;level=1", false).unwrap(),
            ("text", "html", BTreeMap::from([("level", "1")]),)
        );

        // Param with space.
        assert_eq!(
            parse_mime("text/html; level=1", false).unwrap(),
            ("text", "html", BTreeMap::from([("level", "1")]),)
        );

        // Multiple params.
        assert_eq!(
            parse_mime("text/html;level=1;origin=EU", false).unwrap(),
            (
                "text",
                "html",
                BTreeMap::from([("level", "1"), ("origin", "EU")]),
            )
        );

        assert_eq!(
            parse_mime::<&str>("text/plain;q=1", false).unwrap_err(),
            Error::QualityNotAllowed,
        )
    }
}
