#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::errors::Error;
use crate::types::{EventRecord, ReleaseKind};

pub fn validate_create(_env: &Env, record: &EventRecord, _owner: &Address) -> Result<(), Error> {
    match record.release_kind {
        ReleaseKind::Multi(n) if n > 0 => {}
        _ => return Err(Error::InvalidReleaseKind),
    }

    // No floor check: crowdfunding pays milestones out of `remaining_escrow`
    // divided by the milestones left, and never reads the prize floors.
    Ok(())
}
