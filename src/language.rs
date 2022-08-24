use crate::{matches_wildcard, AsNegotiationStr, Error, NegotiationType};

#[derive(Clone, Debug)]
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

    fn parse_sort_header(header: &str) -> Result<Vec<(Self::Parsed, f32)>, Error> {
        let mut languages = header
            .split(',')
            .map(|entry| {
                let mut parts = entry.split(';').map(str::trim);
                let left = parts.next().ok_or(Error::InvalidHeader)?;
                let (main, sub) = left.split_once('-').unwrap_or_else(|| (left, "*"));
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
                Ok(((main.to_owned(), sub.to_owned()), q))
            })
            .collect::<Result<Vec<_>, _>>()?;
        languages.sort_by(|((_, s1), q1), ((_, s2), q2)| {
            q1.total_cmp(q2)
                .then_with(|| (s2 == "*").cmp(&(s1 == "*")))
                .reverse()
        });
        Ok(languages)
    }

    fn is_match(supported: &Self::Parsed, header: &Self::Parsed) -> bool {
        supported.0 == header.0 && matches_wildcard(&supported.1, &header.1)
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
