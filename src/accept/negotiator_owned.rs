use std::iter::Map;
use std::slice::Iter;

use crate::accept::are_wildcards_valid;
use crate::accept::negotiator::Negotiator;
use crate::Error;

#[derive(Debug)]
pub struct NegotiatorOwned {
    supported: Vec<(String, String, String)>,
}

impl NegotiatorOwned {
    pub fn new<I, S>(supported: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Ok(Self {
            supported: supported
                .into_iter()
                .map(|full| {
                    let full = full.into();
                    let (main, sub) = full.split_once('/').ok_or(Error::MissingSeparator)?;
                    if sub.contains('/') {
                        return Err(Error::TooManyPart);
                    }
                    are_wildcards_valid(main, sub)?;
                    let (main, sub) = (main.to_owned(), sub.to_owned());
                    Ok((full, main, sub))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl<'a> Negotiator<'a, 'a> for NegotiatorOwned {
    type SupportedIter = Map<
        Iter<'a, (String, String, String)>,
        fn(&(String, String, String)) -> (&str, &str, &str),
    >;

    fn supported(&'a self) -> Self::SupportedIter {
        self.supported.iter().map(tuple_ref)
    }
}

fn tuple_ref(t: &(String, String, String)) -> (&str, &str, &str) {
    (t.0.as_str(), t.1.as_str(), t.2.as_ref())
}

#[cfg(test)]
mod tests {
    use crate::accept::negotiator::Negotiator;
    use crate::Error;

    use super::NegotiatorOwned;

    #[test]
    fn owned() {
        assert_eq!(
            NegotiatorOwned::new(["text/html"])
                .unwrap()
                .negotiate("text/html"),
            Ok(Some("text/html"))
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            NegotiatorOwned::new(["text"]).unwrap_err(),
            Error::MissingSeparator,
        );
        assert_eq!(
            NegotiatorOwned::new(["text/html/whatever"]).unwrap_err(),
            Error::TooManyPart
        );
        assert_eq!(
            NegotiatorOwned::new(["*/html"]).unwrap_err(),
            Error::InvalidWildcard
        );
    }
}
