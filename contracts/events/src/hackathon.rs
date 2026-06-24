// boundless-events: hackathon-specific behavior.
//
// Spec: boundless-platform-contract-prd.md Section 7.
//
// Hackathons use ReleaseKind::Single and require a submission deadline.
// Open submission model: no per-applicant credit charge.
//
// Wired by create_event dispatch.
#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::errors::Error;
use crate::types::{EventRecord, ReleaseKind};

pub fn validate_create(
    _env: &Env,
    record: &EventRecord,
    _owner: &Address,
) -> Result<(), Error> {
    if !matches!(record.release_kind, ReleaseKind::Single) {
        return Err(Error::InvalidReleaseKind);
    }
    if record.deadline.is_none() {
        return Err(Error::DeadlineRequired);
    }
    Ok(())
}
