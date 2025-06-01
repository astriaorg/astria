<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Fetch and submit transactions already in the rollup transaction pool before submitting
    those coming from `eth_subscribe` [#2086](https://github.com/astriaorg/astria/pull/2086).

### Fixed

- Fix memory leak in metrics [2221](https://github.com/astriaorg/astria/pull/2221).

## [1.0.1] - 2025-03-06

### Changed

- Update `idna` dependency to resolve cargo audit warning [#1869](https://github.com/astriaorg/astria/pull/1869).

## [1.0.0] - 2024-10-25

### Changed

- Bump MSRV to 1.83.0 [#1857](https://github.com/astriaorg/astria/pull/1857).
- Bump penumbra dependencies [#1740](https://github.com/astriaorg/astria/pull/1740).
- Propagate errors [#1838](https://github.com/astriaorg/astria/pull/1838).

## [1.0.0-rc.2] - 2024-10-23

### Changed

- Make native asset optional [#1703](https://github.com/astriaorg/astria/pull/1703).

## [1.0.0-rc.1] - 2024-10-17

### Changed

- Replace `once_cell` with `LazyLock` [#1576](https://github.com/astriaorg/astria/pull/1576).
- Migrate all instances of `#[allow]` to `#[expect]` [#1561](https://github.com/astriaorg/astria/pull/1561).
- Remove action suffix from all action types [#1630](https://github.com/astriaorg/astria/pull/1630).
- Update `futures-util` dependency based on cargo audit warning [#1644](https://github.com/astriaorg/astria/pull/1644).
- Prefer `astria.primitive.v1.RollupId` over bytes [#1661](https://github.com/astriaorg/astria/pull/1661).
- Call transactions `Transaction`, contents `TransactionBody` [#1650](https://github.com/astriaorg/astria/pull/1650).
- Rename sequence action to rollup data submission [#1665](https://github.com/astriaorg/astria/pull/1665).
- Upgrade to proto `v1`s throughout [#1672](https://github.com/astriaorg/astria/pull/1672).

### Fixed

- Update to work with appside mempool [#1643](https://github.com/astriaorg/astria/pull/1643).

## [0.8.3] - 2024-09-06

### Changed

- Improved instrumentation [#1326](https://github.com/astriaorg/astria/pull/1326).

## [0.8.2] - 2024-08-22

### Changed

- Update `bytemark` dependency based on cargo audit warning [#1350](https://github.com/astriaorg/astria/pull/1350).

## [0.8.1] - 2024-07-26

### Added

- Add chain_id check on executor build [#1175](https://github.com/astriaorg/astria/pull/1175).

## [0.8.0] - 2024-06-27

### Added

- Add bech32m addresses [#1124](https://github.com/astriaorg/astria/pull/1124).

### Changed

- Use macro to declare metric constants [#1129](https://github.com/astriaorg/astria/pull/1129).
- Remove non-bech32m address bytes [#1186](https://github.com/astriaorg/astria/pull/1186).
- Use full IBC ICS20 denoms instead of IDs [#1209](https://github.com/astriaorg/astria/pull/1209).

## [0.7.0] - 2024-05-21

### Added

- Add initial set of metrics [#932](https://github.com/astriaorg/astria/pull/932).

### Changed

- Update `SignedTransaction` to contain `Any` for transaction [#1044](https://github.com/astriaorg/astria/pull/1044).
- Avoid holding private key in env var [#1074](https://github.com/astriaorg/astria/pull/1074).

## [0.6.0] - 2024-04-26

### Added

- Add a gRPC collector [#784](https://github.com/astriaorg/astria/pull/784).
- Add graceful shutdown [#854](https://github.com/astriaorg/astria/pull/854).
- Create wrapper types for `RollupId` and `Account` [#987](https://github.com/astriaorg/astria/pull/987).

### Changed

- Interact with executor through handle [#834](https://github.com/astriaorg/astria/pull/834).
- Update to ABCI v0.38 [#831](https://github.com/astriaorg/astria/pull/831).
- Fully split `sequencerapis` and remove [#958](https://github.com/astriaorg/astria/pull/958).
- Require chain id in transactions [#973](https://github.com/astriaorg/astria/pull/973).

### Fixed

- Make snapshot testing deterministic [#865](https://github.com/astriaorg/astria/pull/865).
- Account for fee asset id while estimating sequence action size [#990](https://github.com/astriaorg/astria/pull/990).
- Add capacity to bundle factory [#937](https://github.com/astriaorg/astria/pull/937).
- Use tx hash as hex again [#1014](https://github.com/astriaorg/astria/pull/1014).

## [0.5.0] - 2024-03-19

### Changed

- Simplify emitting error fields with cause chains [#765](https://github.com/astriaorg/astria/pull/765).
- Disambiguate chain-id [#791](https://github.com/astriaorg/astria/pull/791).
- Migrate `v1alpha1` sequencer apis to `v1` [#817](https://github.com/astriaorg/astria/pull/817).
- Rename `Collector` to `GethCollector` [#792](https://github.com/astriaorg/astria/pull/792).
- Flatten module structure [#796](https://github.com/astriaorg/astria/pull/796).

### Fixed

- Reset timer when bundle empty [#804](https://github.com/astriaorg/astria/pull/804).

## [0.4.0] - 2024-02-15

### Added

- Add `SignedTransaction::sha256_of_proto_encoding()` method [#687](https://github.com/astriaorg/astria/pull/687).
- Use opentelemetry [#656](https://github.com/astriaorg/astria/pull/656).
- Metrics setup [#739](https://github.com/astriaorg/astria/pull/739) and [#750](https://github.com/astriaorg/astria/pull/750).
- Add pretty-printing to stdout [#736](https://github.com/astriaorg/astria/pull/736).
- Print build info in all services [#753](https://github.com/astriaorg/astria/pull/753).

### Changed

- Update licenses [#706](https://github.com/astriaorg/astria/pull/706).
- Bundle multiple rollup transactions into a single sequencer transaction [#651](https://github.com/astriaorg/astria/pull/651).
- Move fee asset from `UnsignedTransaction` to `SequenceAction` and
TransferAction` [#719](https://github.com/astriaorg/astria/pull/719).
- Bump rust to 1.76, cargo-chef to 0.1.63 [#744](https://github.com/astriaorg/astria/pull/744).
- Add some information to crates update msrv [#754](https://github.com/astriaorg/astria/pull/754).

### Fixed

- Replace allocating display impl [#738](https://github.com/astriaorg/astria/pull/738).
- Fix docker builds [#756](https://github.com/astriaorg/astria/pull/756).

## [0.3.1] - 2024-01-10

### Added

- Lint debug fields in tracing events [#664](https://github.com/astriaorg/astria/pull/664).

### Changed

- Add proto formatting, cleanup justfile [#637](https://github.com/astriaorg/astria/pull/637).
- Switch tagging format in CI [#639](https://github.com/astriaorg/astria/pull/639).
- Don't deny unknown config fields [#657](https://github.com/astriaorg/astria/pull/657).
- Define abci error codes in protobuf [#647](https://github.com/astriaorg/astria/pull/647).
- Use display formatting instead of debug formatting in tracing events [#671](https://github.com/astriaorg/astria/pull/671).

### Fixed

- Amend Cargo.toml when building images [#672](https://github.com/astriaorg/astria/pull/672).

## [0.3.0] - 2023-11-30

### Added

- Add integrity test of eth tx [#574](https://github.com/astriaorg/astria/pull/574).

### Changed

- Redefine sequencer blocks, celestia blobs as protobuf [#395](https://github.com/astriaorg/astria/pull/395).

## [0.2.5] - 2023-11-07

### Fixed

- Refetch nonce on submission failure [#459](https://github.com/astriaorg/astria/pull/459).
- Fix flaky test [#552](https://github.com/astriaorg/astria/pull/552).

## [0.2.4] - 2023-10-24

### Fixed

- Resubscribe if rollup subscription stops [#532](https://github.com/astriaorg/astria/pull/532).

## [0.2.3] - 2023-10-17

### Added

- Allow rollup names with dash [#514](https://github.com/astriaorg/astria/pull/514).

## [0.2.2] - 2023-10-12

### Added

- Log collected txs [#460](https://github.com/astriaorg/astria/pull/460).
- Report cause of failed nonce fetch [#492](https://github.com/astriaorg/astria/pull/492).

### Changed

- Bump penumbra, tendermint; prune workspace cargo of unused deps [#468](https://github.com/astriaorg/astria/pull/468).

## 0.2.1 - 2023-09-29

### Fixed

- Execute docker builds on new tags [#422](https://github.com/astriaorg/astria/pull/422).

## 0.2.0 - 2023-09-22

### Added

- Initial release.

[unreleased]: https://github.com/astriaorg/astria/compare/composer-v1.0.0...HEAD
[1.0.1]: https://github.com/astriaorg/astria/compare/composer-v1.0.0...composer-v1.0.1
[1.0.0]: https://github.com/astriaorg/astria/compare/composer-v1.0.0-rc.2...composer-v1.0.0
[1.0.0-rc.2]: https://github.com/astriaorg/astria/compare/composer-v1.0.0-rc.1...composer-v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/astriaorg/astria/compare/composer-v0.8.3...composer-v1.0.0-rc.1
[0.8.3]: https://github.com/astriaorg/astria/compare/composer-v0.8.2...composer-v0.8.3
[0.8.2]: https://github.com/astriaorg/astria/compare/composer-v0.8.1...composer-v0.8.2
[0.8.1]: https://github.com/astriaorg/astria/compare/composer-v0.8.0...composer-v0.8.1
[0.8.0]: https://github.com/astriaorg/astria/compare/composer-v0.7.0...composer-v0.8.0
[0.7.0]: https://github.com/astriaorg/astria/compare/composer-v0.6.0...composer-v0.7.0
[0.6.0]: https://github.com/astriaorg/astria/compare/composer-v0.5.0...composer-v0.6.0
[0.5.0]: https://github.com/astriaorg/astria/compare/composer-v0.4.0...composer-v0.5.0
[0.4.0]: https://github.com/astriaorg/astria/compare/composer-v0.3.1...composer-v0.4.0
[0.3.1]: https://github.com/astriaorg/astria/compare/composer-v0.3.0...composer-v0.3.1
[0.3.0]: https://github.com/astriaorg/astria/compare/v0.2.5--composer...v0.3.0--composer
[0.2.5]: https://github.com/astriaorg/astria/compare/v0.2.4--composer...v0.2.5--composer
[0.2.4]: https://github.com/astriaorg/astria/compare/v0.2.3--composer...v0.2.4--composer
[0.2.3]: https://github.com/astriaorg/astria/compare/v0.2.2--composer...v0.2.3--composer
[0.2.2]: https://github.com/astriaorg/astria/compare/v0.2.1--composer...v0.2.2--composer
