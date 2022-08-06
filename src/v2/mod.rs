use std::collections::BTreeMap;

use crate::{extract_quality, matches_wildcard, mime_precision_score, parse_mime, AsMime, Error};

pub trait NegotiationType<'a> {
    type Parsed;
    type ParsedHeader;

    fn parse_elem<M: AsMime>(input: &M) -> Result<Self::Parsed, Error>;

    fn parse_sort_header(header: &'a str) -> Result<Vec<(Self::ParsedHeader, f32)>, Error>;

    fn is_match(supported: &Self::Parsed, header: &Self::ParsedHeader) -> bool;
}

pub struct ContentTypeNegotiation;

impl<'a> NegotiationType<'a> for ContentTypeNegotiation {
    type Parsed = (String, String, BTreeMap<String, String>);
    type ParsedHeader = (&'a str, &'a str, BTreeMap<&'a str, &'a str>);

    fn parse_elem<M: AsMime>(raw: &M) -> Result<Self::Parsed, Error> {
        let (main, sub, params) = parse_mime::<String>(raw.as_mime(), false)?;
        if params.contains_key("q") {
            return Err(Error::QualityNotAllowed);
        }
        Ok((main, sub, params))
    }

    fn parse_sort_header(header: &'a str) -> Result<Vec<(Self::ParsedHeader, f32)>, Error> {
        let mut mimes = header
            .split(',')
            .map(|m| {
                let (main, sub, mut params) = parse_mime::<&str>(m.trim(), true)?;
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

    fn is_match(supported: &Self::Parsed, header: &Self::ParsedHeader) -> bool {
        matches_wildcard(&supported.0, header.0)
            && matches_wildcard(&supported.1, header.1)
            && supported
                .2
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .eq(header.2.iter().map(|(&k, &v)| (k, v)))
    }
}

pub struct Negotiator<'a, N: NegotiationType<'a>, T> {
    supported: Vec<(N::Parsed, T)>,
}

impl<'a, N, T> Negotiator<'a, N, T>
where
    N: NegotiationType<'a>,
{
    pub fn len(&self) -> usize {
        self.supported.len()
    }

    pub fn unwrap_first(&self) -> &T {
        &self.supported[0].1
    }
}

impl<'a, N, T> Negotiator<'a, N, T>
where
    N: NegotiationType<'a>,
    T: AsMime,
{
    fn new<I>(iter: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
    {
        Ok(Self {
            supported: iter
                .into_iter()
                .map(|m| Ok((N::parse_elem(&m)?, m)))
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn negotiate(&self, header: &'a str) -> Result<Option<&T>, Error> {
        for mime in N::parse_sort_header(header)? {
            for (supported_parsed, value) in &self.supported {
                if N::is_match(supported_parsed, &mime.0) {
                    return Ok(Some(value));
                }
            }
        }
        Ok(None)
    }
}
