use crate::Error;

pub mod accept;
pub mod negotiator;

fn parse_mime_with_params(mime_with_params: &str) -> Result<(&str, &str, &str), Error> {
    let mime = mime_with_params
        .split(';')
        .next()
        .ok_or(Error::InvalidHeader)?;
    let (main, sub) = parse_mime(mime)?;
    Ok((mime, main, sub))
}

fn parse_mime(mime: &str) -> Result<(&str, &str), Error> {
    let (main, sub) = mime.split_once('/').ok_or(Error::MissingSeparator)?;
    if sub.contains('/') {
        return Err(Error::TooManyPart);
    }
    if main == "*" && sub != "*" {
        return Err(Error::InvalidWildcard);
    }
    Ok((main, sub))
}

// pub trait Supported {
//     type Origin;
// }
//
// pub struct Ref<'a> {
//     phantom: PhantomData<&'a ()>,
// }
//
// impl<'a> Supported for Ref<'a> {
//     type Origin = &'a str;
// }
