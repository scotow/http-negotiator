use crate::accept::negotiator::Negotiator;
use crate::Error;

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
    type SupportedIter = std::slice::Iter<'b, (&'a str, &'a str, &'a str)>;

    fn supported(&'b self) -> Self::SupportedIter {
        self.supported.iter()
    }
}
