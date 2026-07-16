#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::errors::Error;
use crate::types::{EventRecord, ReleaseKind};

pub fn validate_create(_env: &Env, record: &EventRecord, _owner: &Address) -> Result<(), Error> {
    if !matches!(record.release_kind, ReleaseKind::Single) {
        return Err(Error::InvalidReleaseKind);
    }
    if record.deadline.is_none() {
        return Err(Error::DeadlineRequired);
    }
    Ok(())
}
