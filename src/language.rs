use crate::{match_first, AsNegotiationStr, Error, MaybeWildcard, NegotiationType};

#[derive(Copy, Clone, Debug)]
pub struct LanguageNegotiation;

impl NegotiationType for LanguageNegotiation {
    type Parsed = (String, String);

    fn parse_elem<M: AsNegotiationStr>(input: &M) -> Result<Self::Parsed, Error> {
        let input = input.as_str();
        if input.contains(';') {
            return Err(Error::ParamsNotAllowed);
        }
        input
            .split_once('-')
            .ok_or(Error::MissingSeparator('-'))
            .map(|(main, sub)| (main.to_owned(), sub.to_owned()))
    }

    fn parse_negotiate_header<'a, T>(
        supported: &'a [(Self::Parsed, T)],
        header: &str,
    ) -> Result<Option<&'a T>, Error> {
        let mut languages = header
            .split(',')
            .map(|entry| {
                let mut parts = entry.split(';').map(str::trim);
                let left = parts.next().ok_or(Error::InvalidHeader)?;
                let (main, sub) = left
                    .split_once('-')
                    .map(|(m, s)| (m, MaybeWildcard::Specific(s)))
                    .unwrap_or((left, MaybeWildcard::Wildcard));
                let q = match parts.next() {
                    Some(first_param) => {
                        let (k, v) = first_param.split_once('=').ok_or(Error::InvalidHeader)?;
                        if k != "q" {
                            return Err(Error::ParamsNotAllowed);
                        }
                        v.parse::<f32>()
                            .map_err(|err| Error::InvalidQuality { source: err })?
                    }
                    None => 1.,
                };
                Ok(((main, sub), q))
            })
            .collect::<Result<Vec<_>, _>>()?;
        languages.sort_by(|((_, s1), q1), ((_, s2), q2)| {
            q1.total_cmp(q2)
                .then_with(|| {
                    (matches!(s2, MaybeWildcard::Wildcard))
                        .cmp(&(matches!(s1, MaybeWildcard::Wildcard)))
                })
                .reverse()
        });
        Ok(match_first(
            supported,
            languages.iter().map(|(l, _q)| l),
            |s, h| s.0 == h.0 && h.1.matches(&s.1),
        ))
    }

    #[cfg(feature = "axum")]
    fn associated_header() -> http::header::HeaderName {
        http::header::ACCEPT_LANGUAGE
    }
}

#[cfg(test)]
mod tests {
    use super::LanguageNegotiation;
    use crate::{Error, Negotiator};

    #[test]
    fn new() {
        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US"])
                .unwrap()
                .supported,
            vec![(("en".to_owned(), "US".to_owned()), "en-US")]
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en"]).unwrap_err(),
            Error::MissingSeparator('-')
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US;q=1"]).unwrap_err(),
            Error::ParamsNotAllowed
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US;type=2"]).unwrap_err(),
            Error::ParamsNotAllowed
        );
    }

    #[test]
    fn negotiate() {
        assert!(Negotiator::<LanguageNegotiation, _>::new(["en-US"])
            .unwrap()
            .negotiate("fr-FR")
            .unwrap()
            .is_none());

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US"])
                .unwrap()
                .negotiate("en-US, fr-FR")
                .unwrap(),
            Some(&"en-US")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US"])
                .unwrap()
                .negotiate("fr-FR, en-US")
                .unwrap(),
            Some(&"en-US")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US"])
                .unwrap()
                .negotiate("en, fr")
                .unwrap(),
            Some(&"en-US")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US"])
                .unwrap()
                .negotiate("fr, en")
                .unwrap(),
            Some(&"en-US")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US", "fr-FR"])
                .unwrap()
                .negotiate("fr, en")
                .unwrap(),
            Some(&"fr-FR")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US", "fr-FR"])
                .unwrap()
                .negotiate("en-US; q=1, fr-FR")
                .unwrap(),
            Some(&"en-US")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US", "fr-FR"])
                .unwrap()
                .negotiate("en-US; q=0.9, fr-FR")
                .unwrap(),
            Some(&"fr-FR")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US", "fr-FR"])
                .unwrap()
                .negotiate("en; q=0.8, fr; q=0.9")
                .unwrap(),
            Some(&"fr-FR")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US", "fr-FR"])
                .unwrap()
                .negotiate("en, fr-FR")
                .unwrap(),
            Some(&"fr-FR")
        );

        assert_eq!(
            Negotiator::<LanguageNegotiation, _>::new(["en-US", "fr-FR"])
                .unwrap()
                .negotiate("en;q=1, fr-FR;q=0.9")
                .unwrap(),
            Some(&"en-US")
        );
    }
}
