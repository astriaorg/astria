<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Gauge metric `last_observed_rollup_height` [#2111](https://github.com/astriaorg/astria/pull/2111).

### Fixed

- Fix memory leak in metrics [2221](https://github.com/astriaorg/astria/pull/2221).

## [1.0.2] - 2025-03-06

### Changed

- Update `idna` dependency to resolve cargo audit warning [#1869](https://github.com/astriaorg/astria/pull/1869).
- Support FROST threshold signing using signer nodes. [#1948](https://github.com/astriaorg/astria/pull/1948).

## [1.0.1] - 2024-11-01

### Changed

- Bump MSRV to 1.83.0 [#1857](https://github.com/astriaorg/astria/pull/1857).

### Fixed

- Set `batch_total_settled_value` metric to 0 when no withdrawals are settled [#1778](https://github.com/astriaorg/astria/pull/1768)
- Fixed ICS20 withdrawal source when using channel with more than one
  port/channel combo.[#1768](https://github.com/astriaorg/astria/pull/1768)

## [1.0.0] - 2024-10-25

### Changed

- Bump penumbra dependencies [#1740](https://github.com/astriaorg/astria/pull/1740).

## [1.0.0-rc.2] - 2024-10-23

### Added

- Add `use_compat_address` configuration value [#1671](https://github.com/astriaorg/astria/pull/1671).
- Metric to track total settled funds [#1693](https://github.com/astriaorg/astria/pull/1693).

### Fixed

- Correctly identify rollup return address in ics20 withdrawal actions [#1714](https://github.com/astriaorg/astria/pull/1714).

## [1.0.0-rc.1] - 2024-10-17

### Added

- Add traceability to rollup deposits [#1410](https://github.com/astriaorg/astria/pull/1410).

### Changed

- Pass GRPC and CometBFT clients to consumers directly [#1510](https://github.com/astriaorg/astria/pull/1510).
- Better grpc client construction [#1528](https://github.com/astriaorg/astria/pull/1528).
- Replace `once_cell` with `LazyLock` [#1576](https://github.com/astriaorg/astria/pull/1576).
- Remove action suffix from all action types [#1630](https://github.com/astriaorg/astria/pull/1630).
- Update `futures-util` dependency based on cargo audit warning [#1644](https://github.com/astriaorg/astria/pull/1644).
- Call transactions `Transaction`, contents `TransactionBody` [#1650](https://github.com/astriaorg/astria/pull/1650).
- Rename sequence action to rollup data submission [#1665](https://github.com/astriaorg/astria/pull/1665).
- Upgrade to proto `v1`s throughout [#1672](https://github.com/astriaorg/astria/pull/1672).

### Fixed

- Migrate from `broadcast_tx_commit` to `broadcast_tx_sync` [#1376](https://github.com/astriaorg/astria/pull/1376).
- Fix memo transaction hash encoding [#1428](https://github.com/astriaorg/astria/pull/1428).

## [0.3.0] - 2024-09-06

### Added

- Add instrumentation [#1324](https://github.com/astriaorg/astria/pull/1324).

### Changed

- Enforce withdrawals consumed [#1391](https://github.com/astriaorg/astria/pull/1391).

### Fixed

- Don't fail entire block due to bad withdraw event [#1409](https://github.com/astriaorg/astria/pull/1409).

## [0.2.1] - 2024-08-22

### Changed

- Improve nonce handling [#1292](https://github.com/astriaorg/astria/pull/1292).
- Update `bytemark` dependency based on cargo audit warning [#1350](https://github.com/astriaorg/astria/pull/1350)

## [0.2.0] - 2024-07-26

### Changed

- Move generated contract bindings to crate [#1237](https://github.com/astriaorg/astria/pull/1237).
- Move bridge-unlock memo to core [#1245](https://github.com/astriaorg/astria/pull/1245).
- Refactor startup to a separate subtask and remove balance check from startup [#1190](https://github.com/astriaorg/astria/pull/1190).
- Make bridge unlock memo string [#1244](https://github.com/astriaorg/astria/pull/1244).
- Share code between cli and service [#1270](https://github.com/astriaorg/astria/pull/1270).
- Define bridge memos in proto [#1285](https://github.com/astriaorg/astria/pull/1285).

### Fixed

- Support withdrawer address that differs from bridge address   [#1262](https://github.com/astriaorg/astria/pull/1262).
- Disambiguate return addresses [#1266](https://github.com/astriaorg/astria/pull/1266).
- Fix nonce handling [#1215](https://github.com/astriaorg/astria/pull/1215).
- Don't panic on init [#1281](https://github.com/astriaorg/astria/pull/1281).

## 0.1.0 - 2024-06-27

### Added

- Initial release of EVM Withdrawer.

[unreleased]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v1.0.2...HEAD
[1.0.2]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v1.0.1...bridge-withdrawer-v1.0.2
[1.0.1]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v1.0.0...bridge-withdrawer-v1.0.1
[1.0.0]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v1.0.0-rc.2...bridge-withdrawer-v1.0.0
[1.0.0-rc.2]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v1.0.0-rc.1...bridge-withdrawer-v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v0.3.0...bridge-withdrawer-v1.0.0-rc.1
[0.3.0]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v0.2.1...bridge-withdrawer-v0.3.0
[0.2.1]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v0.2.0...bridge-withdrawer-v0.2.1
[0.2.0]: https://github.com/astriaorg/astria/compare/bridge-withdrawer-v0.1.0...bridge-withdrawer-v0.2.0
