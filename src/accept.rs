use std::collections::BTreeMap;

use crate::{
    extract_quality, matches_wildcard, mime_precision_score, parse_mime, AsMime, Error,
    NegotiationType,
};

#[derive(Clone, Default, Debug)]
pub struct ContentTypeNegotiation;

impl NegotiationType for ContentTypeNegotiation {
    type Parsed = (String, String, BTreeMap<String, String>);

    fn parse_elem<M: AsMime>(raw: &M) -> Result<Self::Parsed, Error> {
        parse_mime(raw.as_mime(), false)
    }

    fn parse_sort_header(header: &str) -> Result<Vec<(Self::Parsed, f32)>, Error> {
        let mut mimes = header
            .split(',')
            .map(|m| {
                let (main, sub, mut params) = parse_mime::<String>(m.trim(), true)?;
                let q = extract_quality(&mut params)?;
                Ok(((main, sub, params), q))
            })
            .collect::<Result<Vec<_>, _>>()?;

        mimes.sort_by(
            |((main_lhs, sub_lhs, params_lhs), q_lhs), ((main_rhs, sub_rhs, params_rhs), q_rhs)| {
                q_lhs
                    .total_cmp(&q_rhs)
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

    fn is_match(supported: &Self::Parsed, header: &Self::Parsed) -> bool {
        matches_wildcard(&supported.0, &header.0)
            && matches_wildcard(&supported.1, &header.1)
            && supported.2 == header.2
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::ContentTypeNegotiation;
    use crate::{Error, NegotiationType, Negotiator};

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
    fn parse_sort() {
        assert_eq!(
            ContentTypeNegotiation::parse_sort_header(
                "text/*, text/plain, text/plain;format=flowed, */*"
            )
            .unwrap(),
            vec![
                (
                    (
                        "text".to_owned(),
                        "plain".to_owned(),
                        BTreeMap::from([("format".to_owned(), "flowed".to_owned())])
                    ),
                    1.
                ),
                (
                    ("text".to_owned(), "plain".to_owned(), BTreeMap::default()),
                    1.
                ),
                (("text".to_owned(), "*".to_owned(), BTreeMap::default()), 1.),
                (("*".to_owned(), "*".to_owned(), BTreeMap::default()), 1.),
            ]
        );

        assert_eq!(
            ContentTypeNegotiation::parse_sort_header(
                "text/*, text/plain, text/plain;format=flowed, */*"
            )
            .unwrap(),
            vec![
                (
                    (
                        "text".to_owned(),
                        "plain".to_owned(),
                        BTreeMap::from([("format".to_owned(), "flowed".to_owned())])
                    ),
                    1.
                ),
                (
                    ("text".to_owned(), "plain".to_owned(), BTreeMap::default()),
                    1.
                ),
                (("text".to_owned(), "*".to_owned(), BTreeMap::default()), 1.),
                (("*".to_owned(), "*".to_owned(), BTreeMap::default()), 1.),
            ]
        );

        assert_eq!(
            ContentTypeNegotiation::parse_sort_header(
                "text/plain;q=0.2,text/not-plain;q=0.4,text/hybrid"
            )
            .unwrap(),
            vec![
                (
                    ("text".to_owned(), "hybrid".to_owned(), BTreeMap::default()),
                    1.
                ),
                (
                    (
                        "text".to_owned(),
                        "not-plain".to_owned(),
                        BTreeMap::default()
                    ),
                    0.4
                ),
                (
                    ("text".to_owned(), "plain".to_owned(), BTreeMap::default()),
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
