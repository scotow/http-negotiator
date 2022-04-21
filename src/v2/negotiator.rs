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
    pub fn new<S>(supported: S) -> Result<Self, Error>
    where
        S: Into<Cow<'a, str>>,
    {
        Ok(Self {
            supported: vec![T::parse_supported(supported)?],
        })
    }

    pub fn negotiate<'b>(&'a self, header: &'b str) -> Result<Option<&'a str>, Error> {
        T::negotiate(&self.supported, header)
    }
}

pub trait Negotiation<'a> {
    type ParsedSupported;

    fn parse_supported<S>(from: S) -> Result<Self::ParsedSupported, Error>
    where
        S: Into<Cow<'a, str>>;

    fn negotiate<'b, 'c>(
        supported: &'c [Self::ParsedSupported],
        header: &'b str,
    ) -> Result<Option<&'a str>, Error>;
}
