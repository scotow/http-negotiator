use crate::v2::negotiator::Negotiation;
use crate::{matches, quality, Error};
use std::borrow::Cow;

pub struct AcceptNegotiation;

impl<'a> Negotiation<'a> for AcceptNegotiation {
    type ParsedSupported = (Cow<'a, str>, Cow<'a, str>, Cow<'a, str>);

    fn parse_supported<S>(origin: S) -> Result<Self::ParsedSupported, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        let full = origin.into();
        let (main, sub) = match &full {
            Cow::Borrowed(full) => {
                let (main, sub) = full.split_once('/').unwrap();
                (Cow::from(main), Cow::from(sub))
            }
            Cow::Owned(full) => {
                let (main, sub) = full.split_once('/').unwrap();
                (Cow::from(main.to_owned()), Cow::from(sub.to_owned()))
            }
        };

        Ok((full, main, sub))
    }

    fn negotiate<'b, 'c>(
        supported: &'c [Self::ParsedSupported],
        header: &'b str,
    ) -> Result<Option<&'a str>, Error> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use crate::v2::accept::AcceptNegotiation;
    use crate::v2::negotiator::Negotiator;
    use std::borrow::Cow;

    #[test]
    fn negotiate() {
        let negotiator = Negotiator::<AcceptNegotiation>::new("text/html").unwrap();
        negotiator.negotiate("text/html").unwrap().unwrap();
    }
}
