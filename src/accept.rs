use std::collections::BTreeMap;

use crate::{extract_quality, matches, mime_score, parse_mime, AsMime, Error};

#[derive(Debug)]
pub struct AcceptNegotiator<T> {
    supported: Vec<(String, String, BTreeMap<String, String>, T)>,
}

impl<T> AcceptNegotiator<T> {
    pub fn new<I>(iter: I) -> Result<Self, Error>
    where
        T: AsMime,
        I: IntoIterator<Item = T>,
    {
        Ok(Self {
            supported: iter
                .into_iter()
                .map(|m| {
                    let (main, sub, params) = parse_mime::<String>(m.as_mime(), false)?;
                    if params.contains_key("q") {
                        return Err(Error::QualityNotAllowed);
                    }
                    Ok((main, sub, params, m))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn len(&self) -> usize {
        self.supported.len()
    }
}

impl<T> AcceptNegotiator<T> {
    pub fn negotiate(&self, header: &str) -> Result<Option<&T>, Error> {
        let mimes = Self::parse_sort_header(header)?;

        for mime in mimes {
            for supported in &self.supported {
                if matches(&supported.0, mime.0)
                    && matches(&supported.1, mime.1)
                    && supported
                        .2
                        .iter()
                        .map(|(k, v)| (k.as_str(), v.as_str()))
                        .eq(mime.2.iter().map(|(&k, &v)| (k, v)))
                {
                    return Ok(Some(&supported.3));
                }
            }
        }

        Ok(None)
    }

    fn parse_sort_header(
        header: &str,
    ) -> Result<Vec<(&str, &str, BTreeMap<&str, &str>, f32)>, Error> {
        let mut mimes = header
            .split(',')
            .map(|m| {
                let (main, sub, mut params) = parse_mime::<&str>(m.trim(), true)?;
                let q = extract_quality(&mut params)?;
                Ok((main, sub, params, q))
            })
            .collect::<Result<Vec<_>, _>>()?;

        mimes.sort_by(|m1, m2| {
            m1.3.total_cmp(&m2.3)
                .then_with(|| mime_score(&m1.0, &m1.1).cmp(&mime_score(&m2.0, &m2.1)))
                .then_with(|| m1.2.len().cmp(&m2.2.len()))
                .reverse()
        });
        Ok(mimes)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::AcceptNegotiator;

    #[test]
    fn new() {
        assert_eq!(
            AcceptNegotiator::new(["text/plain"]).unwrap().supported,
            vec![(
                "text".to_owned(),
                "plain".to_owned(),
                BTreeMap::default(),
                "text/plain"
            )]
        );
    }

    #[test]
    fn parse_sort() {
        assert_eq!(
            AcceptNegotiator::<&str>::parse_sort_header(
                "text/*, text/plain, text/plain;format=flowed, */*"
            )
            .unwrap(),
            vec![
                ("text", "plain", BTreeMap::from([("format", "flowed")]), 1.),
                ("text", "plain", BTreeMap::default(), 1.),
                ("text", "*", BTreeMap::default(), 1.),
                ("*", "*", BTreeMap::default(), 1.),
            ]
        );

        assert_eq!(
            AcceptNegotiator::<&str>::parse_sort_header(
                "text/*, text/plain, text/plain;format=flowed, */*"
            )
            .unwrap(),
            vec![
                ("text", "plain", BTreeMap::from([("format", "flowed")]), 1.),
                ("text", "plain", BTreeMap::default(), 1.),
                ("text", "*", BTreeMap::default(), 1.),
                ("*", "*", BTreeMap::default(), 1.),
            ]
        );

        assert_eq!(
            AcceptNegotiator::<&str>::parse_sort_header(
                "text/plain;q=0.2,text/not-plain;q=0.4,text/hybrid"
            )
            .unwrap(),
            vec![
                ("text", "hybrid", BTreeMap::default(), 1.),
                ("text", "not-plain", BTreeMap::default(), 0.4),
                ("text", "plain", BTreeMap::default(), 0.2),
            ]
        );
    }

    #[test]
    fn negotiate() {
        assert!(AcceptNegotiator::new(["application/json"])
            .unwrap()
            .negotiate("text/html")
            .unwrap()
            .is_none());

        assert_eq!(
            AcceptNegotiator::new(["application/json"])
                .unwrap()
                .negotiate("application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("audio/mp3, application/json")
                .unwrap(),
            Some(&"application/json")
        );

        assert_eq!(
            AcceptNegotiator::new(["application/json", "text/plain"])
                .unwrap()
                .negotiate("text/plain, application/json")
                .unwrap(),
            Some(&"text/plain")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/html;level=3", "text/html;level=2", "image/jpeg", "text/plain", "text/html", "text/html;level=1"])
                .unwrap()
                .negotiate("text/*;q=0.3, text/html;q=0.7, text/html;level=1, text/html;level=2;q=0.4, */*;q=0.5")
                .unwrap(),
            Some(&"text/html;level=1")
        );

        assert_eq!(
            AcceptNegotiator::new(["text/plain", "application/json"])
                .unwrap()
                .negotiate("text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8")
                .unwrap(),
            Some(&"text/plain")
        );

        assert_eq!(
            AcceptNegotiator::new(["application/json", "text/plain"])
                .unwrap()
                .negotiate("text/plain;q=0.9, */*")
                .unwrap(),
            Some(&"application/json")
        );
    }
}
