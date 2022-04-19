use std::iter::Map;

use crate::accept::negotiator::Negotiator;
use crate::Error;

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
                    let (main, sub) = (main.to_owned(), sub.to_owned());
                    Ok((full, main, sub))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl<'a> Negotiator<'a, 'a> for NegotiatorOwned {
    type SupportedIter = Map<
        std::slice::Iter<'a, (String, String, String)>,
        for<'r> fn(&'r (String, String, String)) -> (&'r str, &'r str, &'r str),
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
    use super::NegotiatorOwned;
    use crate::accept::negotiator::Negotiator;

    #[test]
    fn owned() {
        assert_eq!(
            NegotiatorOwned::new(["text/html"])
                .unwrap()
                .negotiate("text/html"),
            Ok(Some("text/html"))
        );
    }
}
