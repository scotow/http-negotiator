use crate::{matches_wildcard, AsMime, Error, NegotiationType};

#[derive(Clone, Default, Debug)]
pub struct EncodingNegotiation;

impl NegotiationType for EncodingNegotiation {
    type Parsed = String;

    fn parse_elem<M: AsMime>(input: &M) -> Result<Self::Parsed, Error> {
        let input = input.as_mime();
        if input.contains(';') {
            return Err(Error::ParamsNotAllowed);
        }
        if input == "*" {
            return Err(Error::InvalidWildcard);
        }
        Ok(input.to_owned())
    }

    fn parse_sort_header(header: &str) -> Result<Vec<(Self::Parsed, f32)>, Error> {
        Ok(header
            .split(',')
            .map(|entry| {
                let mut parts = entry.split(';').map(str::trim);
                let main = parts.next().ok_or(Error::InvalidHeader)?;
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
                Ok((main.to_owned(), q))
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    fn is_match(supported: &Self::Parsed, header: &Self::Parsed) -> bool {
        matches_wildcard(supported, header)
    }
}

#[cfg(test)]
mod tests {
    use super::EncodingNegotiation;
    use crate::{Error, NegotiationType, Negotiator};

    #[test]
    fn new() {
        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip"])
                .unwrap()
                .supported,
            vec![(("gzip".to_owned()), "gzip")]
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip;q=1"]).unwrap_err(),
            Error::ParamsNotAllowed
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip;type=2"]).unwrap_err(),
            Error::ParamsNotAllowed
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["*"]).unwrap_err(),
            Error::InvalidWildcard
        );
    }

    #[test]
    fn negotiate() {
        assert!(Negotiator::<EncodingNegotiation, _>::new(["gzip"])
            .unwrap()
            .negotiate("compress")
            .unwrap()
            .is_none());

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip"])
                .unwrap()
                .negotiate("compress, gzip")
                .unwrap(),
            Some(&"gzip")
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip"])
                .unwrap()
                .negotiate("compress, gzip")
                .unwrap(),
            Some(&"gzip")
        );
    }
}
