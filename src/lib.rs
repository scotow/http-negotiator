pub use accept::*;
pub use error::Error;

pub mod accept;
mod error;
mod v2;
mod v3;

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
        assert_eq!(super::quality("text/html; q =0.9"), Ok(1.)); // Invalid param ignored.
        assert_eq!(super::quality("text/html; q = 0.9"), Ok(1.)); // Invalid param ignored.
    }
}
