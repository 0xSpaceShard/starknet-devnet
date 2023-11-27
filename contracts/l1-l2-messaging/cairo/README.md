# Cairo Contracts

This folder contains a Scarb package to compile and deploy Cairo 1
contracts on Devnet for development purposes.

## Work with Scarb

Start by installing Scarb (with `asdf` **highly** recommended) [from the tutorial](https://docs.swmansion.com/scarb/).
Ensure you've at least version `2.3.1` installed.

### Build

To build contracts, use:
```bash
scarb build
```

The contracts artifacts are generated into `target/dev` folder.
Two files can be found there:
* The Sierra class file: `package_contract.contract_class.json`
* The compiled CASM file: `package_contract.compiled_contract_class.json`

### Interact with Devnet

To interact with Devnet, [Starkli](https://book.starkli.rs/) is the easiest CLI tool to use.
To work with Starkli, you need two files:
* The keystore file with the private key being encrypted there. This file can also be replaced by the private
  key in plain text, which is totally fine for testing.
* The account file with the account definition and address.
