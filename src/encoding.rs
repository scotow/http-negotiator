use crate::{matches_wildcard, AsNegotiationStr, Error, NegotiationType};

#[derive(Clone, Debug)]
pub struct EncodingNegotiation;

impl NegotiationType for EncodingNegotiation {
    type Parsed = String;

    fn parse_elem<M: AsNegotiationStr>(input: &M) -> Result<Self::Parsed, Error> {
        let input = input.as_str();
        if input.contains(';') {
            return Err(Error::ParamsNotAllowed);
        }
        if input == "*" {
            return Err(Error::InvalidWildcard);
        }
        Ok(input.to_owned())
    }

    fn parse_sort_header(header: &str) -> Result<Vec<(Self::Parsed, f32)>, Error> {
        let mut methods = header
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
            .collect::<Result<Vec<_>, _>>()?;
        methods.sort_by(|(_, q1), (_, q2)| q1.total_cmp(q2).reverse());
        Ok(methods)
    }

    fn is_match(supported: &Self::Parsed, header: &Self::Parsed) -> bool {
        matches_wildcard(supported, header)
    }
}

#[cfg(test)]
mod tests {
    use super::EncodingNegotiation;
    use crate::{Error, Negotiator};

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
            Negotiator::<EncodingNegotiation, _>::new(["gzip", "compress"])
                .unwrap()
                .negotiate("compress, gzip")
                .unwrap(),
            Some(&"compress")
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip", "compress"])
                .unwrap()
                .negotiate("compress; q=1, gzip")
                .unwrap(),
            Some(&"compress")
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip", "compress"])
                .unwrap()
                .negotiate("compress; q=0.9, gzip")
                .unwrap(),
            Some(&"gzip")
        );

        assert_eq!(
            Negotiator::<EncodingNegotiation, _>::new(["gzip", "compress"])
                .unwrap()
                .negotiate("compress; q=0.8, gzip; q=0.9")
                .unwrap(),
            Some(&"gzip")
        );
    }
}
