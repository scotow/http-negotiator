use crate::{matches, quality, Error};

pub trait Negotiator<'a, 'b> {
    type SupportedIter;

    fn supported(&'b self) -> Self::SupportedIter;

    fn negotiate(&'b self, header: &str) -> Result<Option<&'a str>, Error>
    where
        'a: 'b,
        Self::SupportedIter: Iterator<Item = &'b (&'a str, &'a str, &'a str)>,
    {
        let mut selected: Option<(&str, f32)> = None;
        for entry in header.split(",").map(|m| m.trim()) {
            let req_full = entry.split(';').next().ok_or(Error::InvalidHeader)?;
            let (req_main, req_sub) = req_full.split_once('/').ok_or(Error::MissingSeparator)?;
            if req_main == "*" && req_sub != "*" {
                return Err(Error::InvalidWildcard);
            }
            let quality = quality(entry)?;

            for (full, main, sub) in self.supported() {
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
