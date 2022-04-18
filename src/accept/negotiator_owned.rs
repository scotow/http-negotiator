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

// impl<'a> Negotiator<'a> for NegotiatorOwned {
//     fn supported(&self) -> &[(&'a str, &'a str, &'a str)] {
//         &self.supported
//     }
// }
