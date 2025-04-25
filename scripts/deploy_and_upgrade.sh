#!/bin/bash

# Script for deploying and upgrading the Boundless contract
# Usage: ./deploy_and_upgrade.sh [deploy|upgrade] [network] [source_account]

# Default values
ACTION=${1:-"deploy"}
NETWORK=${2:-"testnet"}
SOURCE_ACCOUNT=${3:-"admin"}
# CONTRACT_DIR="boundless/contracts/boundless"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if stellar CLI is installed
if ! command -v stellar &> /dev/null; then
    echo -e "${RED}Error: stellar CLI is not installed.${NC}"
    echo "Please install it from https://soroban.stellar.org/docs/getting-started/setup"
    exit 1
fi

# Function to deploy the contract
deploy_contract() {
    echo -e "${YELLOW}Deploying Boundless contract to ${NETWORK}...${NC}"
    
    # Build the contract with reference-types enabled
    echo "Building contract..."
    cd $CONTRACT_DIR || { echo -e "${RED}Failed to change directory to $CONTRACT_DIR${NC}"; exit 1; }
    
    # Use stellar contract build which handles the reference-types issue
    stellar contract build
    
    # Deploy the contract
    echo "Deploying contract..."
    CONTRACT_ID=$(stellar contract deploy \
        --wasm target/wasm32-unknown-unknown/release/boundless.wasm \
        --source $SOURCE_ACCOUNT \
        --network $NETWORK)
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Deployment failed!${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Contract deployed successfully!${NC}"
    echo "Contract ID: $CONTRACT_ID"
    
    # Create .stellar directory if it doesn't exist
    mkdir -p .stellar
    
    # Save the contract ID to a file for future reference
    echo $CONTRACT_ID > .stellar/contract_id_${NETWORK}.txt
    echo "Contract ID saved to .stellar/contract_id_${NETWORK}.txt"
    
    # Get the admin address
    ADMIN_ADDRESS=$(stellar keys address $SOURCE_ACCOUNT)
    echo "Admin address: $ADMIN_ADDRESS"
    
    # Initialize the contract
    echo "Initializing contract..."
    stellar contract invoke \
        --id $CONTRACT_ID \
        --source $SOURCE_ACCOUNT \
        --network $NETWORK \
        -- \
        initialize \
        --admin $ADMIN_ADDRESS
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Initialization failed!${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Contract initialized successfully!${NC}"
}

# Function to upgrade the contract
upgrade_contract() {
    echo -e "${YELLOW}Upgrading Boundless contract on ${NETWORK}...${NC}"
    
    # Check if contract ID file exists
    if [ ! -f .stellar/contract_id_${NETWORK}.txt ]; then
        echo -e "${RED}Contract ID file not found. Please deploy the contract first.${NC}"
        exit 1
    fi
    
    # Read the contract ID
    CONTRACT_ID=$(cat .stellar/contract_id_${NETWORK}.txt)
    
    # Build the contract with reference-types enabled
    echo "Building contract..."
    cd $CONTRACT_DIR || { echo -e "${RED}Failed to change directory to $CONTRACT_DIR${NC}"; exit 1; }
    
    # Use stellar contract build which handles the reference-types issue
    stellar contract build
    
    # Install the new WASM
    echo "Installing new WASM..."
    WASM_HASH=$(stellar contract install \
        --source-account $SOURCE_ACCOUNT \
        --wasm target/wasm32-unknown-unknown/release/boundless.wasm \
        --network $NETWORK)
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}WASM installation failed!${NC}"
        exit 1
    fi
    
    echo "WASM hash: $WASM_HASH"
    
    # Upgrade the contract
    echo "Upgrading contract..."
    stellar contract invoke \
        --id $CONTRACT_ID \
        --source $SOURCE_ACCOUNT \
        --network $NETWORK \
        -- \
        upgrade \
        --new_wasm_hash $WASM_HASH
    
    if [ $? -ne 0 ]; then
        echo -e "${RED}Upgrade failed!${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Contract upgraded successfully!${NC}"
    echo "Contract ID: $CONTRACT_ID"
}

# Main script logic
case $ACTION in
    "deploy")
        deploy_contract
        ;;
    "upgrade")
        upgrade_contract
        ;;
    *)
        echo -e "${RED}Invalid action. Use 'deploy' or 'upgrade'.${NC}"
        echo "Usage: ./deploy_and_upgrade.sh [deploy|upgrade] [network] [source_account]"
        exit 1
        ;;
esac

echo -e "${GREEN}Operation completed successfully!${NC}" 
