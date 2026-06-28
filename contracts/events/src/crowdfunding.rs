// boundless-events: crowdfunding-specific behavior.
//
// Spec: boundless-crowdfunding-prd.md.
//
// Crowdfunding is a builder-led, community-funded pillar with these rules:
//   - ReleaseKind::Multi(n>0): milestones drive release cadence, like grants.
//   - deadline required: defines the funding window. Submitting/contributing
//     after the deadline is rejected by the standard event-active checks.
//   - Owner is the project builder. There is exactly one recipient, also
//     the builder, registered as Winner at position 1 (100% of distribution).
//   - No upfront owner deposit: create_event SKIPS escrow::deposit_with_fee
//     for Pillar::Crowdfunding. Escrow starts at 0 and grows via add_funds
//     from community backers.
//   - winner_distribution MUST be a single entry at position 1 with 100%.
//   - claim_milestone uses dynamic math:
//         amount = remaining_escrow / (total_milestones - claimed_so_far)
//     so each release pays a fair share of whatever the campaign actually
//     raised. The first milestone takes 1/n of escrow, the next 1/(n-1) of
//     what's left, ..., the last takes the entire remainder.
//
// The contract enforces only the on-chain shape. Off-chain layers add admin
// review, community voting, milestone validation, and pause semantics.
#![allow(dead_code)]

use soroban_sdk::{Address, Env};

use crate::errors::Error;
use crate::types::{EventRecord, ReleaseKind};

pub fn validate_create(_env: &Env, record: &EventRecord, _owner: &Address) -> Result<(), Error> {
    // Multi(n) required.
    match record.release_kind {
        ReleaseKind::Multi(n) if n > 0 => {}
        _ => return Err(Error::InvalidReleaseKind),
    }

    // Funding window required.
    if record.deadline.is_none() {
        return Err(Error::DeadlineRequired);
    }

    // Distribution must be exactly one entry at position 1 with 100%. The
    // builder is the sole recipient; no co-recipient splits are supported.
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
