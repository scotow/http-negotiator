pub use negotiator::Negotiator as AcceptNegotiator;
pub use negotiator_owned::NegotiatorOwned as AcceptNegotiatorOwned;
pub use negotiator_ref::NegotiatorRef as AcceptNegotiatorRef;

use crate::Error;

mod negotiator;
mod negotiator_owned;
mod negotiator_ref;

fn are_wildcards_valid(main: &str, sub: &str) -> Result<(), Error> {
    if main == "*" && sub != "*" {
        return Err(Error::InvalidWildcard);
    }
    Ok(())
}
