# The NFT Standard on Casper (CEP-47)

[CEP-47](https://github.com/casper-ecosystem/casper-nft-cep47) is the NFT standard for the Casper blockchain, defining a minimum interface contract code should implement to manage, own, and trade unique tokens on the Casper Network. 

The Casper NFT standard takes full advantage of [unforgeable references](https://docs.casperlabs.io/design/uref/) to store values and manage permissions to them. It also takes advantage of other access control features (such as [groups](https://docs.casperlabs.io/glossary/G/#groups)). We recommend exploring the [main functions](/Basic-Tutorial.md#casper-nft-cep-47-functions) of the [contract](https://github.com/casper-ecosystem/casper-nft-cep47/blob/master/cep47/bin/cep47_token.rs) to understand the standard further.

The equivalent NFT standard on Ethereum is [ERC-721](https://eips.ethereum.org/EIPS/eip-721).

For more information on contract implementation and sending the contract to the network, visit the [CEP-47 Basic Tutorial](/Basic-Tutorial.md), an illustrated guide on implementing, deploying, and testing a sample Casper NFT contract.

Visit the [Contract Interaction and Events Tutorial](/Contract-Interaction-Tutorial.md) to get more details about emitting and monitoring contract events.

# Enhanced

## Permissions

When install a new CEP47

```
name
symbol
meta
whitelist_accounts Empty to allow all accounts, set to allow specific accounts which are able to call mint / update metadata
whitelist_contracts: Empty to reject all contracts, set to allow specific contracts which are able to call mint / update metadata
```
 
## Merge
 
- token_ids: List ofif id of token to merge, the LAST one will be kept
- check_prop: Property to verify 2 tokens which are able to merge, they must have the same value (e.g Class: Rock) and the caller must be also the owner of tokens.
 
# Errors

```
PermissionDenied = 1
WrongArguments = 2
TokenIdAlreadyExists = 3
TokenIdDoesntExist = 4
InvalidKey = 69
UnlistedContractHash = 81
MissingAdminRights = 204
MissingMintRights = 205
MissingMetadataRights = 206
InvalidLength = 207
MissingCheckingProperty = 208
MissingMetadata = 209
MissingMetadataValue = 210
DifferentMetadata = 211
```
 
