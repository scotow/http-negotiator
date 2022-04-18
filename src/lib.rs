mod error;

pub use error::Error;

pub struct Negotiator<'a> {
    supported: Vec<(&'a str, &'a str, &'a str)>,
}

impl<'a> Negotiator<'a> {
    pub fn new(supported: &[&'a str]) -> Result<Self, Error> {
        Ok(Self {
            supported: supported
                .into_iter()
                .map(|&full| {
                    let (main, sub) = full.split_once('/').ok_or(Error::MissingSeparator)?;
                    Ok((full, main, sub))
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn negotiate(&self, header: &str) -> Result<Option<&'a str>, Error> {
        let mut selected: Option<(&str, f32)> = None;
        for entry in header.split(",").map(|m| m.trim()) {
            let req_full = entry.split(';').next().ok_or(Error::InvalidHeader)?;
            let (req_main, req_sub) = req_full.split_once('/').ok_or(Error::MissingSeparator)?;
            if req_main == "*" && req_sub != "*" {
                return Err(Error::InvalidWildcard);
            }
            let quality = quality(entry)?;

            for &(full, main, sub) in &self.supported {
                if let Some((_, prev_quality)) = selected {
                    if prev_quality > quality {
                        continue;
                    }
                }
                if matches(main, req_main) && matches(sub, req_sub) {
                    selected = Some((full, quality));
                    break;
                }
            }
        }
        Ok(selected.map(|s| s.0))
    }
}

fn quality(mime: &str) -> Result<f32, Error> {
    for param in mime.split(';').map(|p| p.trim()) {
        let (k, v) = match param.split_once('=') {
            Some(r) => r,
            None => continue,
        };
        if k != "q" {
            continue;
        }
        return Ok(v
            .parse::<f32>()
            .map_err(|err| Error::InvalidQuality { source: err })?);
    }
    Ok(1.)
}

fn matches(left: &str, right: &str) -> bool {
    left == right || right == "*"
}

#[cfg(test)]
mod tests {
    use super::Error;
    use super::Negotiator;

    fn negotiate<'a, const N: usize>(
        header: &str,
        supported: [&'a str; N],
    ) -> Result<Option<&'a str>, Error> {
        Negotiator::new(&supported).unwrap().negotiate(header)
    }

    #[test]
    fn negotiation() {
        // No match.
        assert_eq!(negotiate("text/html", ["application/json"]), Ok(None));
        // One to one match.
        assert_eq!(negotiate("text/html", ["text/html"]), Ok(Some("text/html")));
        // Multiple to one match.
        assert_eq!(
            negotiate("application/json, text/html", ["text/html"]),
            Ok(Some("text/html"))
        );
        // One to multiple.
        assert_eq!(
            negotiate("text/html", ["application/json", "text/html"]),
            Ok(Some("text/html"))
        );
        // Subtype wildcard.
        assert_eq!(
            negotiate("text/*", ["application/json", "text/html"]),
            Ok(Some("text/html"))
        );
        // Full wildcard.
        assert_eq!(
            negotiate("*/*", ["application/json", "text/html"]),
            Ok(Some("application/json"))
        );
        // Weighted header.
        assert_eq!(
            negotiate(
                "text/html, application/json, application/xml;q=0.9",
                ["application/xml"]
            ),
            Ok(Some("application/xml"))
        );
        // Full wildcard + weighted header.
        assert_eq!(
            negotiate(
                "text/html, application/xhtml+xml, application/xml;q=0.9, */*;q=0.8",
                ["application/json"]
            ),
            Ok(Some("application/json")),
        );
        // Un-ordered weighted header.
        assert_eq!(
            negotiate(
                "text/html;q=0.8, application/json;q=0.9",
                ["text/html", "application/json"]
            ),
            Ok(Some("application/json")),
        );
    }

    #[test]
    fn quality() {
        assert_eq!(super::quality("text/html"), Ok(1.));
        assert_eq!(super::quality("text/html;q=1"), Ok(1.));
        assert_eq!(super::quality("text/html;q=0.9"), Ok(0.9));
        assert_eq!(super::quality("text/html;q=0.95"), Ok(0.95));
        assert_eq!(super::quality("text/html; q=0.9"), Ok(0.9));
        assert!(super::quality("text/html; q= 0.9")
            .unwrap_err()
            .is_invalid_quality());
        assert_eq!(super::quality("text/html; q =0.9"), Ok(1.));
        assert_eq!(super::quality("text/html; q = 0.9"), Ok(1.));
    }
}
