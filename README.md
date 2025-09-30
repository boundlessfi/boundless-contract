# Boundless Project

## Overview
Boundless is a Soroban-based smart contract project that implements a comprehensive decentralized platform on the Stellar blockchain network. The platform supports multiple funding models including campaigns, grants, and hackathons, all with milestone-based tracking and management. 

**Integration with Trustless Work**: Boundless leverages [Trustless Work](http://trustlesswork.com) for secure, milestone-based stablecoin escrow payments, while our smart contract maintains comprehensive records and tracking of all activities. This hybrid approach provides the security of Trustless Work's escrow system with our detailed record-keeping and management capabilities.

## Contract Details
- **Contract ID**: `CDBSJXFA4J4PALS2FKJYS22UEBDBOBDYLPJYRFPBEF6MLMJI7DOHSEE4`
- **Network**: Testnet
- **Status**: Deployed and Active
- **Deployment Date**: Updated

You can view the contract on Stellar Expert: [View Contract](https://stellar.expert/explorer/testnet/contract/CDBSJXFA4J4PALS2FKJYS22UEBDBOBDYLPJYRFPBEF6MLMJI7DOHSEE4)

## Key Features

### Multi-Entity Support
- **Campaigns**: Traditional crowdfunding with milestone-based funding
- **Grants**: Grant programs with application tracking and winner selection
- **Hackathons**: Competition events with judging and prize distribution

### Core Functionality
- **Milestone Management**: Generic milestone system for all entity types
- **Lifecycle Management**: Complete entity lifecycle (create, complete, cancel)
- **Participant Tracking**: Backers, applicants, entries, and judges
- **Winner Selection**: Automated winner selection for grants and hackathons
- **Record Keeping**: Comprehensive tracking and state management
- **Admin Controls**: Administrative oversight for all operations
- **Trustless Work Integration**: Seamless escrow coordination for secure payments

## Prerequisites
- Rust (latest stable version)
- Soroban CLI
- Stellar development environment

## Project Structure
```
.
├── contracts/
│   └── boundless/           # Main contract implementation
│       ├── src/
│       │   ├── lib.rs       # Contract entry point
│       │   ├── interface.rs # Contract interface definitions
│       │   ├── datatypes.rs # Data structures and types
│       │   └── logic/       # Core business logic modules
│       │       ├── mod.rs   # Module declarations
│       │       ├── admin.rs # Admin operations
│       │       ├── campaign.rs # Campaign management
│       │       ├── grant.rs # Grant management
│       │       ├── hackathon.rs # Hackathon management
│       │       └── milestone.rs # Milestone operations
│       ├── Cargo.toml      # Contract dependencies
│       └── Makefile        # Build automation
├── docs/                    # Documentation
│   └── CONTRACT.md         # Detailed contract documentation
├── CONTRIBUTING.md         # Contribution guidelines
├── CONTRIBUTORS.md         # List of contributors
└── README.md
```

## Getting Started

### Installation
1. Clone the repository:
   ```bash
   git clone git@github.com:boundlessfi/boundless-contract.git
   cd boundless
   ```

2. Build the project:
   ```bash
   cd contracts/boundless
   make build
   ```

### Development
- New contracts should be added to the `contracts` directory
- Each contract has its own directory with a `Cargo.toml` file
- Contracts inherit dependencies from the workspace-level `Cargo.toml`

## Smart Contracts
The project uses Soroban SDK for smart contract development. The main contract implements:

### Entity Management
- **Campaign Management**: Create, fund, and track crowdfunding campaigns
- **Grant Management**: Handle grant applications and winner selection
- **Hackathon Management**: Organize competitions with judging and scoring

### Core Operations
- **Milestone Management**: Generic milestone system for all entity types
- **Lifecycle Management**: Complete entity lifecycle control
- **Participant Tracking**: Manage backers, applicants, entries, and judges
- **Admin Operations**: Administrative controls and oversight
- **Record Keeping**: Comprehensive tracking and state management
- **Escrow Coordination**: Integration with Trustless Work for secure payments

### Building Contracts
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Testing
```bash
cargo test
```

### Deployment
The contract is automatically deployed using GitHub Actions workflows. The deployment process includes:
1. Code verification and formatting checks
2. Building the contract
3. Deploying to the Stellar testnet
4. Upgrading existing contracts when needed

For manual deployment, use the following command:
```bash
./deploy_and_upgrade.sh deploy testnet YOUR_SECRET_KEY
```

## Configuration
The project uses the following optimization settings for release builds:
- Maximum optimization level
- Overflow checks enabled
- Debug symbols stripped
- Link-time optimization enabled
- Single codegen unit for better optimization

## Contract Architecture

### Data Types
- **Entity Types**: Campaign, Grant, Hackathon
- **Status Management**: Active, Completed, Failed, Cancelled
- **Milestone Status**: Pending, Approved, Rejected, Released
- **Participant Types**: Backers, Applicants, Entries, Judges

### Storage Keys
- Entity collections (Campaigns, Grants, Hackathons)
- Individual entity data
- Entity-specific participant data
- Admin and system configuration

### Error Handling
- Comprehensive error types for all operations
- Soroban-compatible error reporting
- Clear error messages for debugging

## Trustless Work Integration

### What We Track
Our smart contract maintains comprehensive records of:
- **Entity Lifecycle**: Campaign, grant, and hackathon states and transitions
- **Participant Data**: Backers, applicants, entries, judges, and their activities
- **Milestone Progress**: Status tracking and approval workflows
- **Winner Selection**: Grant and hackathon winner determination
- **Admin Actions**: Administrative oversight and control operations

### What Trustless Work Handles
[Trustless Work](http://trustlesswork.com) provides:
- **Secure Escrow**: Non-custodial milestone-based payments
- **Stablecoin Support**: Multi-chain stablecoin escrow capabilities
- **Payment Security**: Audited escrow smart contracts
- **API Integration**: Easy integration with existing applications

### Hybrid Architecture
- **Boundless**: Record keeping, participant management, lifecycle control
- **Trustless Work**: Secure payment escrows, fund custody, milestone releases
- **Combined**: Complete platform with security and comprehensive tracking

## Contributing
We welcome contributions from everyone! Please read our [Contributing Guidelines](CONTRIBUTING.md) for details on how to submit pull requests, report issues, and contribute to the project.

Check out our [Contributors](CONTRIBUTORS.md) page to see the amazing people who have helped make this project possible.

## License
MIT License

## Contact
- GitHub: [@0xdevcollins](https://github.com/0xdevcollins)
- Project Link: [https://github.com/0xdevcollins/boundless](https://github.com/0xdevcollins/boundless)
