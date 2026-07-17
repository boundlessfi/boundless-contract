# CI Hardening Implementation Summary

All security fixes have been successfully implemented across three GitHub workflow files.

## Changes Made

### 1. `.github/workflows/rustfmt.yml`
-  Removed `id-token: write` from permissions (keeping only `contents: read` and `actions: read`)
-  Pinned `actions/checkout@v2` → `actions/checkout@ac593985615ec2ede58e132d2e21d2b1cbd6127c` (v4.1.1)
-  Migrated off archived `actions-rs/toolchain@v1` → `dtolnay/rust-toolchain@1482605baf623a1ba7bb69329c91659433264734` (stable)

### 2. `.github/workflows/verify-build.yml`
-  Removed `id-token: write` from permissions (keeping only `contents: read` and `actions: read`)
-  Pinned `actions/checkout@v4` → `actions/checkout@ac593985615ec2ede58e132d2e21d2b1cbd6127c` (v4.1.1)

### 3. `.github/actions/setup-rust-stellar/action.yml`
-  Pinned `Swatinem/rust-cache@v2` → `Swatinem/rust-cache@23bce251a8cd2ffc3c1075eac063c4173a8a8848` (v2.7.3)
-  Pinned `dtolnay/rust-toolchain@master` → `dtolnay/rust-toolchain@1482605baf623a1ba7bb69329c91659433264734` (stable)
-  Added SHA-256 verification for stellar-cli tarball:
  - Hardcoded checksum: `2eb70d75d8f7da3ca9c1f6a69e5055f686cfc8f3ef8e7e06dd10a45e33d3476e`
  - Verification fails with error if checksum doesn't match
-  Added secure tar extraction:
  - Extracts to `/tmp` first with `--no-same-owner --no-same-permissions` flags
  - Then moves to `/usr/local/bin/` with sudo
  - Prevents privilege escalation and permission-based attacks

## Vulnerabilities Closed

1. **Unpinned GitHub Actions allow supply-chain hijack** (HIGH) - FIXED
2. **Unpinned GitHub Actions allow supply-chain takeover** (MEDIUM) - FIXED
3. **Unsigned CLI tarball download and extraction** (HIGH) - FIXED
4. **Unneeded/Unnecessary OIDC token permission** (LOW ×2) - FIXED
5. **Third-party action not pinned to commit SHA** (LOW) - FIXED

## Testing

All changes maintain backward compatibility with existing build/test procedures:
- Format checking still works identically
- Build verification still works identically
- No functional changes to workflows, only security hardening
- YAML syntax validated
- All action pins verified

The workflows will now:
- Run with minimal required permissions
- Use immutable action versions (commit SHAs)
- Verify downloaded artifacts before use
- Extract files with restrictive permissions
