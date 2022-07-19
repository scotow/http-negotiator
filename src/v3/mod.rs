use crate::Error;
use std::collections::BTreeMap;

mod accept;

fn parse_mime<'a, T: From<&'a str> + Ord>(mime: &'a str) -> Result<(T, T, BTreeMap<T, T>), Error> {
    let mut parts = mime.split(';');
    let left = parts.next().ok_or(Error::InvalidHeader)?.trim();

    let (main, sub) = left.split_once('/').ok_or(Error::MissingSeparator)?;
    if sub.contains('/') {
        return Err(Error::TooManyPart);
    }
    if main == "*" && sub != "*" {
        return Err(Error::InvalidWildcard);
    }
    Ok((
        main.into(),
        sub.into(),
        parts
            .map(|param| {
                let (k, v) = param.trim().split_once('=').ok_or(Error::InvalidHeader)?;
                Ok((k.into(), v.into()))
            })
            .collect::<Result<_, _>>()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_mime;
    use std::collections::BTreeMap;

    #[test]
    fn parse() {
        // Basic.
        assert_eq!(
            parse_mime("text/plain").unwrap(),
            ("text", "plain", BTreeMap::default()),
        );

        // With one param.
        assert_eq!(
            parse_mime("text/html;level=1").unwrap(),
            ("text", "html", BTreeMap::from([("level", "1")]),)
        );

        // Param with space.
        assert_eq!(
            parse_mime("text/html; level=1").unwrap(),
            ("text", "html", BTreeMap::from([("level", "1")]),)
        );

        // Multiple params.
        assert_eq!(
            parse_mime("text/html;level=1;origin=EU").unwrap(),
            (
                "text",
                "html",
                BTreeMap::from([("level", "1"), ("origin", "EU")]),
            )
        );
    }
}
