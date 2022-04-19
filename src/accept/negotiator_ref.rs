use std::iter::Copied;
use std::slice::Iter;

use crate::accept::are_wildcards_valid;
use crate::accept::negotiator::Negotiator;
use crate::Error;

#[derive(Debug)]
pub struct NegotiatorRef<'a> {
    supported: Vec<(&'a str, &'a str, &'a str)>,
}

impl<'a, 'b> NegotiatorRef<'a> {
    pub fn new<I, S>(supported: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a S>,
        S: AsRef<str> + ?Sized + 'a,
    {
        Ok(Self {
            supported: supported
                .into_iter()
                .map(|full| {
                    let (main, sub) = full
                        .as_ref()
                        .split_once('/')
                        .ok_or(Error::MissingSeparator)?;
                    if sub.contains('/') {
                        return Err(Error::TooManyPart);
                    }
                    are_wildcards_valid(main, sub)?;
                    Ok((full.as_ref(), main, sub))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl<'a, 'b> Negotiator<'a, 'b> for NegotiatorRef<'a>
where
    'a: 'b,
{
    type SupportedIter = Copied<Iter<'b, (&'a str, &'a str, &'a str)>>;

    fn supported(&'b self) -> Self::SupportedIter {
        self.supported.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use crate::accept::negotiator::Negotiator;
    use crate::Error;

    use super::NegotiatorRef;

    #[test]
    fn referenced() {
        assert_eq!(
            NegotiatorRef::new(["text/html"])
                .unwrap()
                .negotiate("text/html"),
            Ok(Some("text/html"))
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            NegotiatorRef::new(["text"]).unwrap_err(),
            Error::MissingSeparator,
        );
        assert_eq!(
            NegotiatorRef::new(["text/html/whatever"]).unwrap_err(),
            Error::TooManyPart
        );
        assert_eq!(
            NegotiatorRef::new(["*/html"]).unwrap_err(),
            Error::InvalidWildcard
        );
    }
}
