use crate::Error;
use std::borrow::Cow;

pub struct Negotiator<'a, T>
where
    T: Negotiation<'a>,
{
    supported: Vec<T::ParsedSupported>,
}

impl<'a, T> Negotiator<'a, T>
where
    T: Negotiation<'a>,
{
    pub fn new<I, S>(supported: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: Into<Cow<'a, str>>,
    {
        Ok(Self {
            supported: supported
                .into_iter()
                .map(|s| T::parse_supported(s))
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn negotiate(&self, header: &str) -> Result<Option<&str>, Error> {
        T::negotiate(&self.supported, header)
    }
}

pub trait Negotiation<'a> {
    type ParsedSupported;

    fn parse_supported<S>(from: S) -> Result<Self::ParsedSupported, Error>
    where
        S: Into<Cow<'a, str>>;

    fn negotiate<'b>(
        supported: &'b [Self::ParsedSupported],
        header: &str,
    ) -> Result<Option<&'b str>, Error>;
}
