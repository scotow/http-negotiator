use std::iter::Map;
use std::slice::Iter;

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

// impl<'a, 'b> Negotiator<'a, 'b> for NegotiatorOwned
// where
//     'a: 'b,
// {
//     type SupportedIter = Box<dyn Iterator<Item = (&str, &str, &str)>>;

//     fn supported(&'b self) -> Self::SupportedIter {

//         Box::new(
//             self.supported
//                 .iter()
//                 .map(|(f, m, s)| (f.as_str(), m.as_str(), s.as_str())),
//         )
//     }
// }

// #[cfg(test)]
// mod tests {
//     use crate::accept::negotiator::Negotiator;

//     use super::NegotiatorOwned;

//     #[test]
//     fn owned() {
//         let negotiator = NegotiatorOwned::new(["text/html"]).unwrap();
//         negotiator.negotiate("text/html");
//     }
// }
