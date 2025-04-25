# Boundless Contract Documentation

## Contract Information
- **Contract ID**: `CDXHCANXIMVQGLP2QFJKBOCYH5VNOEN5D75EGXBU7D7JOFZFF3JTR4ET`
- **Network**: Stellar Testnet
- **Explorer Link**: [View on Stellar Expert](https://stellar.expert/explorer/testnet/contract/CDXHCANXIMVQGLP2QFJKBOCYH5VNOEN5D75EGXBU7D7JOFZFF3JTR4ET)

## Contract Overview
The Boundless contract is a Soroban-based smart contract that implements a decentralized crowdfunding platform with milestone-based funding releases. It allows creators to create projects, receive funding, and release funds through milestone achievements that are voted on by the community.

## Key Features
- Project-based crowdfunding
- Milestone-based fund release
- Community voting system
- Refund mechanism for failed projects
- Admin-controlled milestone approval

## Contract Functions

### Project Management
```rust
fn create_project(env: Env, project_id: String, creator: Address, metadata_uri: String, funding_target: u64, milestone_count: u32)
fn get_project(env: Env, project_id: String) -> Project
fn update_project_metadata(env: Env, project_id: String, creator: Address, new_metadata_uri: String)
fn update_project_milestone_count(env: Env, project_id: String, creator: Address, new_milestone_count: u32)
fn close_project(env: Env, project_id: String, creator: Address)
fn list_projects(env: Env) -> Vec<String>
```

### Funding Operations
```rust
fn fund_project(env: Env, project_id: String, amount: i128, funder: Address, token_contract: Address)
fn refund(env: Env, project_id: String, token_contract: Address)
fn get_project_funding(env: Env, project_id: String) -> (u64, u64)
fn get_backer_contribution(env: Env, project_id: String, backer: Address) -> u64
```

### Voting Operations
```rust
fn vote_project(env: Env, project_id: String, voter: Address, vote_value: i32)
fn withdraw_vote(env: Env, project_id: String, voter: Address)
fn has_voted(env: Env, project_id: String, voter: Address) -> bool
fn get_vote(env: Env, project_id: String, voter: Address) -> i32
```

### Milestone Operations
```rust
fn release_milestone(env: Env, project_id: String, milestone_number: u32, admin: Address)
fn approve_milestone(env: Env, project_id: String, milestone_number: u32, admin: Address)
fn reject_milestone(env: Env, project_id: String, milestone_number: u32, admin: Address)
fn get_milestone_status(env: Env, project_id: String, milestone_number: u32) -> MilestoneStatus
fn get_project_milestones(env: Env, project_id: String) -> Vec<Milestone>
```

## Project Lifecycle
1. **Creation**: Projects are created with a funding target and milestone count
2. **Funding Period**: 30-day period for backers to fund the project
3. **Voting Period**: 30-day period for community voting
4. **Milestone Execution**: Sequential release and approval of milestones
5. **Completion/Refund**: Successful completion or refund process

## Data Structures

### Project Status
```rust
enum ProjectStatus {
    Funding = 1,    // Project is in funding phase
    Voting = 2,     // Project is in voting phase
    Funded = 3,     // Project has been successfully funded
    Failed = 4,     // Project funding failed
    Closed = 5,     // Project has been closed by creator
}
```

### Milestone Status
```rust
enum MilestoneStatus {
    Pending,    // Not released yet
    Released,   // Released for approval
    Approved,   // Approved by admin
    Rejected    // Rejected by admin
}
```

## Security Considerations
1. Admin-controlled milestone approval system
2. Time-locked funding periods (30 days)
3. Vote withdrawal capability
4. Refund mechanism for failed projects
5. Creator verification for project modifications

## Error Handling
The contract defines specific errors for various scenarios:
- `AlreadyInitialized`: Contract initialization attempts
- `Unauthorized`: Permission-related operations
- `ProjectClosed`: Operations on closed projects
- `FundingPeriodEnded`: Late funding attempts
- `VotingPeriodEnded`: Late voting attempts
- `InsufficientFunds`: Funding-related issues
- `InvalidOperation`: Status-dependent operations

## Constants
- Funding Period: 30 days
- Voting Period: 30 days
- Day in Ledgers: 17,280 (assuming 5 seconds per ledger)
- Project Bump Amount: 30 days worth of ledgers
- Project Lifetime Threshold: 29 days worth of ledgers

## Deployment Information

### Testnet Deployment
The contract is deployed on the Stellar testnet with the following details:
- Contract ID: `CDXHCANXIMVQGLP2QFJKBOCYH5VNOEN5D75EGXBU7D7JOFZFF3JTR4ET`
- Deployment Date: 25/04/2025
- Latest Update: 25/04/2025

### Deployment Process
The contract is deployed using GitHub Actions with the following workflow:
1. Code verification and formatting checks
2. Contract building and optimization
3. Deployment to testnet
4. Contract upgrade (when needed)

## Testing
The contract includes comprehensive tests that can be run using:
```bash
cargo test
```

## Contributing
[Add guidelines for contributing to the contract development] 
