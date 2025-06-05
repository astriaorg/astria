<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Initial release.
- Add method `TracePrefixed::leading_channel` to read the left-most channel of
  a trace prefixed ICS20 asset [#1768](https://github.com/astriaorg/astria/pull/1768).
- Add `impl Protobuf for Address<Bech32m>` [#1802](https://github.com/astriaorg/astria/pull/1802).
- Add module `sequencerblock::optimistic::v1alpha1`
  and `impl Protobuf for FilteredSequencerBlock` [#1839](https://github.com/astriaorg/astria/pull/1839).
- Add `BridgeTransfer` action and `BridgeTransfer` variant to `FeeChange`
  [#1934](https://github.com/astriaorg/astria/pull/1934).
- Add `upgrades` module with various types to support sequencer network upgrades
  including the first upgrade named `Aspen` [#2085](https://github.com/astriaorg/astria/pull/2085).
- Add `oracles/price_feed` module with various types to support including
  price feed data in sequencer blocks [#2085](https://github.com/astriaorg/astria/pull/2085).
- Add `block::DataItem` enum to hold the initial "injected" elements of the
  `txs` in CometBFT requests/responses [#2085](https://github.com/astriaorg/astria/pull/2085).
- Add `block::ExpandedBlockData` to hold the parsed elements of the `data` field
  of a sequencer block, along with values derived from these [#2085](https://github.com/astriaorg/astria/pull/2085).
- Add `block::ExtendedCommitInfoWithProof` and associated types to allow
  sequencers to utilize CometBFT's extended votes for getting consensus on price
  feed data [#2085](https://github.com/astriaorg/astria/pull/2085).
- Add `name` field to domain `ValidatorUpdate` action [#2089](https://github.com/astriaorg/astria/pull/2089).
- Add new ABCI error code 18: `TRANSACTION_INCLUDED_IN_BLOCK` [#2133](https://github.com/astriaorg/astria/pull/2133).
- Add new ABCI error code 19: `TRANSACTION_FAILED_CHECK_TX` [#2133](https://github.com/astriaorg/astria/pull/2133).
- Add new ABCI error code 20: `INVALID_TRANSACTION_BYTES` [#2133](https://github.com/astriaorg/astria/pull/2133).
- Add new ABCI error code 21: `INVALID_TRANSACTION` [#2133](https://github.com/astriaorg/astria/pull/2133).
- Add `CheckedTransaction`, `CheckedAction` and `Checked...` wrappers for all
  action types [#2142](https://github.com/astriaorg/astria/pull/2142).
- Add `Blackburn` upgrade [#2228](https://github.com/astriaorg/astria/pull/2228).

### Changed

- Bump MSRV to 1.83.0 [#1857](https://github.com/astriaorg/astria/pull/1857).
- Move `astria_core::crypto` to `astria-core-crypto` and reexport
  `astria_core_crypto as crypto` (this change is transparent)
  [#1800](https://github.com/astriaorg/astria/pull/1800/).
- Move definitions of address domain type to `astria-core-address` and
  reexported items using the same aliases [#1802](https://github.com/astriaorg/astria/pull/1802).
- Move all Astria APIs generated from the Protobuf spec from
  `astria_core::generated` to `astria_core::generated::astria`
  [#1825](https://github.com/astriaorg/astria/pull/1825).
- Update `idna` dependency to resolve cargo audit warning [#1869](https://github.com/astriaorg/astria/pull/1869).
- Replaced all instances of `[u8; 32]` by newtype
  `astria_core::sequencerblock::v1::block::Hash` where appropriate [#1884](https://github.com/astriaorg/astria/pull/1884).
- Replace `execution::v1` with `execution::v2` API [#2006](https://github.com/astriaorg/astria/pull/2006).
- Change `ExpandedBlockData::user_submitted_transactions` to UN-parsed
  transactions so that all checking can be done when constructing a
  `CheckedTransaction` from this [#2142](https://github.com/astriaorg/astria/pull/2142).
- Rename ABCI error code 10 from `TRANSACTION_FAILED` to
  `TRANSACTION_FAILED_EXECUTION` [#2142](https://github.com/astriaorg/astria/pull/2142).
- Change `CurrencyPairsChange` action to hold an `IndexSet` rather than a `Vec`
  of currency pairs [#2171](https://github.com/astriaorg/astria/pull/2171).

### Removed

- Remove method `TracePrefixed::last_channel` [#1768](https://github.com/astriaorg/astria/pull/1768).
- Remove method `SigningKey::try_address` [#1800](https://github.com/astriaorg/astria/pull/1800/).
- Remove inherent methods `Address::try_from_raw` and `Address::to_raw`
  [#1802](https://github.com/astriaorg/astria/pull/1802).
- Remove `AddressBuilder::with_iter` from public interface [#1802](https://github.com/astriaorg/astria/pull/1802).
