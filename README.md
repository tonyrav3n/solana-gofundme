# Solana GoFundMe

A decentralized crowdfunding smart contract built on Solana using the Anchor framework.

## Overview

This repository contains the backend (on-chain program) for a GoFundMe-style crowdfunding application on Solana. It allows users to create fundraisers, accept donations, withdraw collected funds, and process refunds if a fundraiser is abandoned past its deadline.

## Features

- **Create Fundraisers**: Initialize a fundraiser with a title, description, target goal amount, and a deadline.
- **Donate**: Users can securely donate SOL to active fundraisers.
- **Withdraw Funds**: Creators can withdraw the collected funds at any time.
- **Refunds**: If a fundraiser is abandoned (i.e., the creator does not withdraw funds within 7 days after the deadline), donors can reclaim their contributions.

## Prerequisites

Ensure you have the following installed on your machine:
- [Rust](https://www.rust-lang.org/tools/install)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor Framework](https://www.anchor-lang.com/docs/installation)
- [Bun](https://bun.sh/) (Package manager)

## Quick Start

1. **Clone the repository:**
   ```bash
   git clone https://github.com/tonyrav3n/solana-gofundme.git
   cd solana-gofundme
   ```

2. **Install dependencies:**
   ```bash
   bun install
   ```

3. **Build the program:**
   ```bash
   anchor build
   ```

4. **Run the tests:**
   To run the integration tests against a local validator:
   ```bash
   anchor test
   ```

## Program Structure

- `programs/gofundme/src/instructions/`: Core logic for all smart contract operations (`initialize_fundraiser`, `donate`, `withdraw`, `process_refunds`).
- `programs/gofundme/src/state/`: On-chain data structures (`Fundraiser`, `Donation`).
- `tests/`: Comprehensive TypeScript integration tests to verify the program's behavior.
