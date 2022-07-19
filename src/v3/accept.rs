use crate::v3::parse_mime;
use crate::Error;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Negotiator<T> {
    supported: Vec<(String, String, BTreeMap<String, String>, T)>,
}

impl<T: AsMime> Negotiator<T> {
    pub fn new<I: IntoIterator<Item = T>>(iter: I) -> Result<Self, Error> {
        Ok(Self {
            supported: iter
                .into_iter()
                .map(|m| {
                    let (main, sub, params) = parse_mime::<String>(m.as_mime())?;
                    if params.contains_key("q") {
                        return Err(Error::QualityNotAllowed);
                    }
                    Ok((main, sub, params, m))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

pub trait AsMime {
    fn as_mime(&self) -> &str;
}

impl AsMime for &str {
    fn as_mime(&self) -> &str {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::Negotiator;
    use crate::Error;
    use std::collections::BTreeMap;

    #[test]
    fn new() {
        assert_eq!(
            Negotiator::new(["text/plain"]).unwrap().supported,
            vec![(
                "text".to_owned(),
                "plain".to_owned(),
                BTreeMap::default(),
                "text/plain"
            )]
        );

        assert_eq!(
            Negotiator::new(["text/plain;q=1"]).unwrap_err(),
            Error::QualityNotAllowed,
        )
    }
}
