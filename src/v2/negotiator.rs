use crate::Error;

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
    pub fn new<I>(supported: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = &'a str>,
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

    fn parse_supported(from: &'a str) -> Result<Self::ParsedSupported, Error>;

    fn negotiate<'b>(
        supported: &'b [Self::ParsedSupported],
        header: &str,
    ) -> Result<Option<&'b str>, Error>;
}
