use std::{borrow::Borrow, collections::BTreeMap};

use crate::{
    extract_quality, match_first, AsNegotiationStr, Error, MaybeWildcard, NegotiationType,
};

#[derive(Copy, Clone, Debug)]
pub struct ContentTypeNegotiation;

impl NegotiationType for ContentTypeNegotiation {
    type Parsed = (String, String, BTreeMap<String, String>);

    fn parse_elem<M: AsNegotiationStr>(raw: &M) -> Result<Self::Parsed, Error> {
        parse_mime(raw.as_str(), false)
    }

    fn parse_negotiate_header<'a, T>(
        supported: &'a [(Self::Parsed, T)],
        header: &str,
    ) -> Result<Option<&'a T>, Error> {
        let mimes = parse_sort_header(header)?;
        Ok(match_first(
            supported,
            mimes.iter().map(|(ct, _q)| ct),
            |s, h| {
                h.0.matches(&s.0) && h.1.matches(&s.1)
                    && s.2
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .eq(h.2.iter().map(|(k, v)| (*k, *v)))
            },
        ))
    }

    #[cfg(feature = "axum")]
    fn associated_header() -> http::header::HeaderName {
        http::header::ACCEPT
    }
}

fn parse_mime<'a, W, T>(mime: &'a str, from_header: bool) -> Result<(W, W, BTreeMap<T, T>), Error>
where
    W: From<&'a str>,
    T: From<&'a str> + Ord + Borrow<str>,
{
    let mut parts = mime.split(';');
    let left = parts.next().ok_or(Error::InvalidHeader)?.trim();

    let (main, sub) = left.split_once('/').ok_or(Error::MissingSeparator('/'))?;
    if sub.contains('/') {
        return Err(Error::TooManyParts);
    }
    if from_header {
        if main == "*" && sub != "*" {
            return Err(Error::InvalidWildcard);
        }
    } else if main == "*" || sub == "*" {
        return Err(Error::InvalidWildcard);
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

#[allow(clippy::type_complexity)]
fn parse_sort_header(
    header: &str,
) -> Result<
    Vec<(
        (
            MaybeWildcard<&str>,
            MaybeWildcard<&str>,
            BTreeMap<&str, &str>,
        ),
        f32,
    )>,
    Error,
> {
    let mut mimes = header
        .split(',')
        .map(|m| {
            let (main, sub, mut params) = parse_mime::<MaybeWildcard<&str>, &str>(m.trim(), true)?;
            let q = extract_quality(&mut params)?;
            Ok(((main, sub, params), q))
        })
        .collect::<Result<Vec<_>, _>>()?;
    mimes.sort_by(
        |((main_lhs, sub_lhs, params_lhs), q_lhs), ((main_rhs, sub_rhs, params_rhs), q_rhs)| {
            q_lhs
                .total_cmp(q_rhs)
                .then_with(|| {
                    mime_precision_score(main_lhs, sub_lhs)
                        .cmp(&mime_precision_score(main_rhs, sub_rhs))
                })
                .then_with(|| params_lhs.len().cmp(&params_rhs.len()))
                .reverse()
        },
    );
    Ok(mimes)
}

fn mime_precision_score(main: &MaybeWildcard<&str>, sub: &MaybeWildcard<&str>) -> u8 {
    match (main, sub) {
        (MaybeWildcard::Wildcard, MaybeWildcard::Wildcard) => 0,
        (_, MaybeWildcard::Wildcard) => 1,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{parse_mime, parse_sort_header, ContentTypeNegotiation};
    use crate::{Error, MaybeWildcard, Negotiator};

    #[test]
    fn new() {
        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["text/plain"])
                .unwrap()
                .supported,
            vec![(
                ("text".to_owned(), "plain".to_owned(), BTreeMap::default()),
                "text/plain"
            )]
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["text/plain;q=1"]).unwrap_err(),
            Error::QualityNotAllowed
        )
    }

    #[test]
    fn parse() {
        // Basic.
        assert_eq!(
            parse_mime::<_, &str>("text/plain", false).unwrap(),
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
            parse_mime::<&str, &str>("text/plain;q=1", false).unwrap_err(),
            Error::QualityNotAllowed,
        );

        assert_eq!(
            parse_mime::<&str, &str>("*/plain", true).unwrap_err(),
            Error::InvalidWildcard
        );

        assert_eq!(
            parse_mime::<&str, &str>("text/*", false).unwrap_err(),
            Error::InvalidWildcard
        );

        assert!(parse_mime::<&str, &str>("text/*", true).is_ok());

        assert_eq!(
            parse_mime::<&str, &str>("text/plain/extra", true).unwrap_err(),
            Error::TooManyParts
        );
    }

    #[test]
    fn parse_sort() {
        assert_eq!(
            parse_sort_header("text/*, text/plain, text/plain;format=flowed, */*").unwrap(),
            vec![
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("plain").into(),
                        BTreeMap::from([("format", "flowed")])
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("plain"),
                        BTreeMap::default()
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Wildcard,
                        BTreeMap::default()
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Wildcard,
                        MaybeWildcard::Wildcard,
                        BTreeMap::default()
                    ),
                    1.
                ),
            ]
        );

        assert_eq!(
            parse_sort_header("text/*, text/plain, text/plain;format=flowed, */*").unwrap(),
            vec![
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("plain"),
                        BTreeMap::from([("format", "flowed")])
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("plain"),
                        BTreeMap::default()
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Wildcard,
                        BTreeMap::default()
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Wildcard,
                        MaybeWildcard::Wildcard,
                        BTreeMap::default()
                    ),
                    1.
                ),
            ]
        );

        assert_eq!(
            parse_sort_header("text/plain;q=0.2,text/not-plain;q=0.4,text/hybrid").unwrap(),
            vec![
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("hybrid"),
                        BTreeMap::default()
                    ),
                    1.
                ),
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("not-plain"),
                        BTreeMap::default()
                    ),
                    0.4
                ),
                (
                    (
                        MaybeWildcard::Specific("text"),
                        MaybeWildcard::Specific("plain"),
                        BTreeMap::default()
                    ),
                    0.2
                ),
            ]
        );
    }

    #[test]
    fn negotiate() {
        assert!(
            Negotiator::<ContentTypeNegotiation, _>::new(["application/json"])
                .unwrap()
                .negotiate("text/html")
                .unwrap()
                .is_none()
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["application/json"])
                .unwrap()
                .negotiate("application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("audio/mp3, application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["application/json", "text/plain"])
                .unwrap()
                .negotiate("text/plain, application/json")
                .unwrap(),
            Some(&"text/plain")
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["text/html;level=3", "text/html;level=2", "image/jpeg", "text/plain", "text/html", "text/html;level=1"])
                .unwrap()
                .negotiate("text/*;q=0.3, text/html;q=0.7, text/html;level=1, text/html;level=2;q=0.4, */*;q=0.5")
                .unwrap(),
            Some(&"text/html;level=1")
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .unwrap(),
            Some(&"text/plain")
        );

        assert_eq!(
            Negotiator::<ContentTypeNegotiation, _>::new(["application/json", "text/plain"])
                .unwrap()
                .negotiate("text/plain;q=0.9, */*")
                .unwrap(),
            Some(&"application/json")
        );
    }
}
