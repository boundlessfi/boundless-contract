# Boundless Project

## Overview
Boundless is a Soroban-based smart contract project that implements a decentralized crowdfunding platform on the Stellar blockchain network. The platform enables creators to launch projects with milestone-based funding, allowing for transparent and accountable fund management through community voting and milestone approvals.

## Contract Details
- **Contract ID**: `CDXHCANXIMVQGLP2QFJKBOCYH5VNOEN5D75EGXBU7D7JOFZFF3JTR4ET`
- **Network**: Testnet
- **Status**: Deployed and Active
- **Deployment Date**: 25/04/2025

You can view the contract on Stellar Expert: [View Contract](https://stellar.expert/explorer/testnet/contract/CDXHCANXIMVQGLP2QFJKBOCYH5VNOEN5D75EGXBU7D7JOFZFF3JTR4ET)

## Key Features
- Create and manage crowdfunding projects
- Milestone-based fund release system
- Community voting mechanism
- Automated refund process for failed projects
- Admin-controlled milestone approvals
- Time-locked funding and voting periods

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
│       │   └── logic.rs     # Core business logic
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
   git clone https://github.com/0xdevcollins/boundless.git
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
- Project management functions
- Funding operations
- Voting system
- Milestone management
- Refund mechanisms

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

## Contract Constants
- Funding Period: 30 days
- Voting Period: 30 days
- Day in Ledgers: 17,280 (5 seconds per ledger)
- Project Lifetime: 30 days

## Contributing
We welcome contributions from everyone! Please read our [Contributing Guidelines](CONTRIBUTING.md) for details on how to submit pull requests, report issues, and contribute to the project.

Check out our [Contributors](CONTRIBUTORS.md) page to see the amazing people who have helped make this project possible.

## License
MIT License

## Contact
- GitHub: [@0xdevcollins](https://github.com/0xdevcollins)
- Project Link: [https://github.com/0xdevcollins/boundless](https://github.com/0xdevcollins/boundless)
