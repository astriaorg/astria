<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fix memory leak in metrics [2221](https://github.com/astriaorg/astria/pull/2221).

## [1.0.1] - 2025-03-06

### Changed

- Update `idna` dependency to resolve cargo audit warning [#1869](https://github.com/astriaorg/astria/pull/1869).

## [1.0.0] - 2024-10-25

### Changed

- Bump MSRV to 1.83.0 [#1857](https://github.com/astriaorg/astria/pull/1857).
- Bump penumbra dependencies [#1740](https://github.com/astriaorg/astria/pull/1740).

## [1.0.0-rc.2] - 2024-10-23

### Changed

- Make native asset optional [#1703](https://github.com/astriaorg/astria/pull/1703).

## [1.0.0-rc.1] - 2024-10-17

### Changed

- Replace `once_cell` with `LazyLock` [#1576](https://github.com/astriaorg/astria/pull/1576).
- Migrate all instances of `#[allow]` to `#[expect]` [#1561](https://github.com/astriaorg/astria/pull/1561).
- Code freeze through github actions [#1588](https://github.com/astriaorg/astria/pull/1588).
- Prefer `astria.primitive.v1.RollupId` over bytes [#1661](https://github.com/astriaorg/astria/pull/1661).
- Upgrade to proto `v1`s throughout [#1672](https://github.com/astriaorg/astria/pull/1672).

## [0.16.2] - 2024-09-06

### Changed

- Improved Instrumentation [#1375](https://github.com/astriaorg/astria/pull/1375).

## [0.16.1] - 2024-08-22

### Fixed

- Cargo audit warning [#1350](https://github.com/astriaorg/astria/pull/1350)
- Change `reqwest` for `isahc` in relayer blackbox tests (ENG-699) [#1366](https://github.com/astriaorg/astria/pull/1366).

## [0.16.0] - 2024-07-26

### Changed

- Minimize resubmissions to Celestia [#1234](https://github.com/astriaorg/astria/pull/1234).

## [0.15.0] - 2024-06-27

### Added

- Add chain IDs for sequencer and Celestia to config env vars [#1063](https://github.com/astriaorg/astria/pull/1063).
- Add bech32m addresses [#1124](https://github.com/astriaorg/astria/pull/1124).

### Changed

- Update `metric_name` macro to handle a collection of names [#1163](https://github.com/astriaorg/astria/pull/1163).
- Add timeout to gRPCs to Celestia app [#1191](https://github.com/astriaorg/astria/pull/1191).
- Remove non-bech32m address bytes [#1186](https://github.com/astriaorg/astria/pull/1186).

### Removed

- Remove functionality to restrict relaying blocks to only those proposed by a
given validator [#1168](https://github.com/astriaorg/astria/pull/1168).

### Fixed

- Ensure tracer providers are shut down in all services [#1098](https://github.com/astriaorg/astria/pull/1098).
- Avoid hanging while waiting for submitter task to return [#1206](https://github.com/astriaorg/astria/pull/1206).

## [0.14.0] - 2024-05-21

### Changed

- Update `SignedTransaction` to contain `Any` for transaction [#1044](https://github.com/astriaorg/astria/pull/1044).
- Make per submission gauges into histograms [#1060](https://github.com/astriaorg/astria/pull/1060).
- Change compression ratio calculation [#1075](https://github.com/astriaorg/astria/pull/1075).

## [0.13.0] - 2024-05-09

### Added

- Provide filter for rollups [#1001](https://github.com/astriaorg/astria/pull/1001).
- Add metric recording highest sequencer block submitted [#1040](https://github.com/astriaorg/astria/pull/1040).

### Changed

- Reinstate black box tests [#1033](https://github.com/astriaorg/astria/pull/1033).
- Batch multiple Sequencer blocks to save on Celestia fees [#1045](https://github.com/astriaorg/astria/pull/1045).

## [0.12.0] - 2024-04-26

### Added

- Provide a shutdown controller [#889](https://github.com/astriaorg/astria/pull/889).
- Brotli compress data blobs [#1006](https://github.com/astriaorg/astria/pull/1006).

### Changed

- Generate pbjon impls for sequencer types needed to mock conductor [#905](https://github.com/astriaorg/astria/pull/905).
- Replace hex by base64 for display formatting, emitting tracing events [#908](https://github.com/astriaorg/astria/pull/908).
- Update `SequencerBlockHeader` and related proto types to not use cometbft
header [#830](https://github.com/astriaorg/astria/pull/830).
- Update to ABCI v0.38 [#831](https://github.com/astriaorg/astria/pull/831).
- Submit blobs directly to celestia app [#963](https://github.com/astriaorg/astria/pull/963).

### Removed

- Remove `SequencerBlock::try_from_cometbft` [#1005](https://github.com/astriaorg/astria/pull/1005).

### Fixed

- Fix shutdown not propagating on API server fail [#883](https://github.com/astriaorg/astria/pull/883).

## [0.11.0] - 2024-03-19

### Added

- Add sequencer service proto [#701](https://github.com/astriaorg/astria/pull/701).
- Add metrics [#771](https://github.com/astriaorg/astria/pull/771).
- Introduce state to not submit previous sequencer blocks [#778](https://github.com/astriaorg/astria/pull/778).
- Warn if submission tracking is inconsistent, but continue operation [#798](https://github.com/astriaorg/astria/pull/798).
- Report information on sequencer block submitted to Celestia [#807](https://github.com/astriaorg/astria/pull/807).
- Implement graceful shutdown [#823](https://github.com/astriaorg/astria/pull/823).

### Changed

- Simplify emitting error fields with cause chains [#765](https://github.com/astriaorg/astria/pull/765).
- Retry sequencer block fetch, celestia blob submission [#769](https://github.com/astriaorg/astria/pull/769).
- Provide address, not port, to serve API requests [#776](https://github.com/astriaorg/astria/pull/776).
- Use Celestia crates published on crates.io [#806](https://github.com/astriaorg/astria/pull/806).
- Make block to filtered block conversion more flexible [#794](https://github.com/astriaorg/astria/pull/794).
- Update relayer to use sequencer API to pull blocks [#810](https://github.com/astriaorg/astria/pull/810).
- Migrate `v1alpha1` sequencer apis to `v1` [#817](https://github.com/astriaorg/astria/pull/817).
- Replace grab-bag constructor with config-like builder [#822](https://github.com/astriaorg/astria/pull/822).

### Removed

- Delete unused proto file [#783](https://github.com/astriaorg/astria/pull/783).

## [0.10.0] - 2024-02-15

### Added

- Add `ibc_sudo_address` to genesis, only allow `IbcRelay` actions from this
address [#721](https://github.com/astriaorg/astria/pull/721).
- Use opentelemetry [#656](https://github.com/astriaorg/astria/pull/656).
- Allow specific assets for fee payment [#730](https://github.com/astriaorg/astria/pull/730).
- Metrics setup [#739](https://github.com/astriaorg/astria/pull/739) and [#750](https://github.com/astriaorg/astria/pull/750).
- Add `ibc_relayer_addresses` list and allow modifications via
`ibc_sudo_address` [#737](https://github.com/astriaorg/astria/pull/737).
- Add pretty-printing to stdout [#736](https://github.com/astriaorg/astria/pull/736).
- Set permitted commitment spread from rollup genesis [#743](https://github.com/astriaorg/astria/pull/743).
- Implement ability to update fee assets using sudo key [#752](https://github.com/astriaorg/astria/pull/752).
- Print build info in all services [#753](https://github.com/astriaorg/astria/pull/753).

### Changed

- Transfer fees to block proposer instead of burning [#690](https://github.com/astriaorg/astria/pull/690).
- Update licenses [#706](https://github.com/astriaorg/astria/pull/706).
- Update balance queries to return every asset owned by account [#683](https://github.com/astriaorg/astria/pull/683).
- Use `IbcComponent` and penumbra `HostInterface`  [#700](https://github.com/astriaorg/astria/pull/700).
- Move fee asset from `UnsignedTransaction` to `SequenceAction` and
`TransferAction` [#719](https://github.com/astriaorg/astria/pull/719).
- Split protos into multiple buf repos [#732](https://github.com/astriaorg/astria/pull/732).
- Add fee for `Ics20Withdrawal` action [#733](https://github.com/astriaorg/astria/pull/733).
- Bump rust to 1.76, cargo-chef to 0.1.63 [#744](https://github.com/astriaorg/astria/pull/744).
- Upgrade to penumbra release 0.66 [#741](https://github.com/astriaorg/astria/pull/741).
- Move ibc-related code to its own module [#757](https://github.com/astriaorg/astria/pull/757).

### Fixed

- Fix `FungibleTokenPacketData` decoding [#686](https://github.com/astriaorg/astria/pull/686).
- Relax size requirements of hash buffers [#709](https://github.com/astriaorg/astria/pull/709).
- Build all binaries, fix pr title ci [#728](https://github.com/astriaorg/astria/pull/728).
- Replace allocating display impl [#738](https://github.com/astriaorg/astria/pull/738).

## [0.9.1] - 2024-01-10

### Added

- Lint debug fields in tracing events [#664](https://github.com/astriaorg/astria/pull/664).

### Changed

- Move protobuf specs to repository top level [#629](https://github.com/astriaorg/astria/pull/629).
- Add proto formatting, cleanup justfile [#637](https://github.com/astriaorg/astria/pull/637).
- Bump all checkout actions in CI to v3 [#6414](https://github.com/astriaorg/astria/pull/6414).
- Switch tagging format in CI [#639](https://github.com/astriaorg/astria/pull/639).
- Autocut release binaries [#643](https://github.com/astriaorg/astria/pull/643).
- Update to use new chart structure, dusk-2 [#611](https://github.com/astriaorg/astria/pull/611).
- Break up sequencer `v1alpha1` module [#646](https://github.com/astriaorg/astria/pull/646).
- Don't deny unknown config fields [#657](https://github.com/astriaorg/astria/pull/657).
- Define abci error codes in protobuf [#647](https://github.com/astriaorg/astria/pull/647).
- Use display formatting instead of debug formatting in tracing events [#671](https://github.com/astriaorg/astria/pull/671).

### Fixed

- Adjust input to proto breaking change linter after refactor [#635](https://github.com/astriaorg/astria/pull/635).
- Amend Cargo.toml when building images [#672](https://github.com/astriaorg/astria/pull/672).

## [0.9.0] - 2023-11-30

### Changed

- Replace buf-generate by tonic_build [#581](https://github.com/astriaorg/astria/pull/581).
- Bump all dependencies (mainly penumbra, celestia, tendermint) [#582](https://github.com/astriaorg/astria/pull/582).
- Require `chain_id` be 32 bytes [#436](https://github.com/astriaorg/astria/pull/436).
- Update penumbra-ibc features [#615](https://github.com/astriaorg/astria/pull/615).
- Redefine sequencer blocks, celestia blobs as protobuf [#395](https://github.com/astriaorg/astria/pull/395).

### Fixed

- update `wait_for_sequencer` log to be correct [#586](https://github.com/astriaorg/astria/pull/586).

## [0.8.0] - 2023-11-18

## Added

- Add an RFC 6962 compliant Merkle tree with flat memory representation [#554](https://github.com/astriaorg/astria/pull/554).

### Fixed

- Don't use fixed fee pricing [#590](https://github.com/astriaorg/astria/pull/590).

## [0.7.0] - 2023-11-14

### Changed

- Implement clippy pedantic suggestions [#572](https://github.com/astriaorg/astria/pull/572).

### Fixed

- Use sequencer chain id for sequencer blobs [#577](https://github.com/astriaorg/astria/pull/577).

## [0.6.0] - 2023-11-07

### Changed

- Sequencer-validation: implement std Error [#430](https://github.com/astriaorg/astria/pull/430).
- Bump penumbra, tendermint; prune workspace cargo of unused deps [#468](https://github.com/astriaorg/astria/pull/468).
- Use fork of tendermint with backported reqwest client [#498](https://github.com/astriaorg/astria/pull/498).
- Celestia-client: use eiger's version [#486](https://github.com/astriaorg/astria/pull/486).
- Define service configs in terms of a central crate [#537](https://github.com/astriaorg/astria/pull/537).
- Remove signing and signature verification of data posted to DA [#538](https://github.com/astriaorg/astria/pull/538).
- Verify current block commit in conductor; remove `last_commit` from
`SequencerBlockData` [#560](https://github.com/astriaorg/astria/pull/560).

### Fixed

- Implement `chain_ids_commitment` inclusion proof generation and verification [#548](https://github.com/astriaorg/astria/pull/548).

## [0.5.1] - 2023-09-27

### Fixed

- Fix tendermint block to `SequencerBlockData` conversion [#424](https://github.com/astriaorg/astria/pull/424).

## 0.5.0 - 2023-09-22

### Added

- Initial release.

[unreleased]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v1.0.1...HEAD
[1.0.1]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v1.0.0...sequencer-relayer-v1.0.1
[1.0.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v1.0.0-rc.2...sequencer-relayer-v1.0.0
[1.0.0-rc.2]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v1.0.0-rc.1...sequencer-relayer-v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.16.2...sequencer-relayer-v1.0.0-rc.1
[0.16.2]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.16.1...sequencer-relayer-v0.16.2
[0.16.1]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.16.0...sequencer-relayer-v0.16.1
[0.16.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.15.0...sequencer-relayer-v0.16.0
[0.15.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.14.0...sequencer-relayer-v0.15.0
[0.14.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.13.0...sequencer-relayer-v0.14.0
[0.13.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.12.0...sequencer-relayer-v0.13.0
[0.12.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.11.0...sequencer-relayer-v0.12.0
[0.11.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.10.0...sequencer-relayer-v0.11.0
[0.10.0]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.9.1...sequencer-relayer-v0.10.0
[0.9.1]: https://github.com/astriaorg/astria/compare/sequencer-relayer-v0.9.0...sequencer-relayer-v0.9.1
[0.9.0]: https://github.com/astriaorg/astria/compare/v0.8.0--sequencer-relayer...v0.9.0--sequencer-relayer
[0.8.0]: https://github.com/astriaorg/astria/compare/v0.7.0--sequencer-relayer...v0.8.0--sequencer-relayer
[0.7.0]: https://github.com/astriaorg/astria/compare/v0.6.0--sequencer-relayer...v0.7.0--sequencer-relayer
[0.6.0]: https://github.com/astriaorg/astria/compare/v0.5.1--sequencer-relayer...v0.6.0--sequencer-relayer
[0.5.1]: https://github.com/astriaorg/astria/compare/v0.5.0--sequencer-relayer...v0.5.1--sequencer-relayer
