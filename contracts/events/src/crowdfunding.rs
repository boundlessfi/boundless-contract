#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::errors::Error;
use crate::types::{EventRecord, ReleaseKind};

pub fn validate_create(_env: &Env, record: &EventRecord, _owner: &Address) -> Result<(), Error> {
    match record.release_kind {
        ReleaseKind::Multi(n) if n > 0 => {}
        _ => return Err(Error::InvalidReleaseKind),
    }

    if record.deadline.is_none() {
        return Err(Error::DeadlineRequired);
    }

    if record.winner_distribution.len() != 1 {
        return Err(Error::InvalidDistribution);
    }
    let percent = record
        .winner_distribution
        .get(1)
        .ok_or(Error::InvalidDistribution)?;
    if percent != 100 {
        return Err(Error::DistributionMismatch);
    }

    Ok(())
}
