use crate::accept::negotiator::Negotiator;
use crate::Error;

pub struct NegotiatorRef<'a> {
    supported: Vec<(&'a str, &'a str, &'a str)>,
}

impl<'a> NegotiatorRef<'a> {
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

impl<'a> Negotiator<'a> for NegotiatorRef<'a> {
    fn supported(&self) -> &[(&'a str, &'a str, &'a str)] {
        &self.supported
    }
}
