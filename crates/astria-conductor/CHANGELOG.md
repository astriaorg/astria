<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fix memory leak in metrics [2221](https://github.com/astriaorg/astria/pull/2221).

## [2.0.0-rc.2] - 2025-05-08

### Fixed

- Fix TLS errors when connecting to remote seqeuncer networks [#2140](https://github.com/astriaorg/astria/pull/2140).

## [2.0.0-rc.1] - 2025-04-22

### Added

- Include price feed oracle data in transactions when sequencer network
  provides it [#2085](https://github.com/astriaorg/astria/pull/2085).

### Changed

- Upgrade to `astria.execution.v2` APIs, implement execution sessions, remove env
  vars for setting chain IDs [#2006](https://github.com/astriaorg/astria/pull/2006).

### Fixed

- Update `crossbeam-channel` dependency to resolve cargo audit warning [#2106](https://github.com/astriaorg/astria/pull/2106).

## [1.1.0] - 2025-03-06

### Changed

- Update `idna` dependency to resolve cargo audit warning [#1869](https://github.com/astriaorg/astria/pull/1869).
- Remove panic source on shutdown [#1919](https://github.com/astriaorg/astria/pull/1919).

### Added

- Send `sequencer_block_hash` as part of `ExecuteBlockRequest` [#1999](https://github.com/astriaorg/astria/pull/1999).

## [1.0.0] - 2024-10-25

### Changed

- Bump MSRV to 1.83.0 [#1857](https://github.com/astriaorg/astria/pull/1857).
- Bump penumbra dependencies [#1740](https://github.com/astriaorg/astria/pull/1740).

## [1.0.0-rc.2] - 2024-10-23

### Changed

- Make native asset optional [#1703](https://github.com/astriaorg/astria/pull/1703).

## [1.0.0-rc.1] - 2024-10-17

### Added

- Add traceability to rollup deposits [#1410](https://github.com/astriaorg/astria/pull/1410).
- Implement restart logic [#1463](https://github.com/astriaorg/astria/pull/1463).
- Implement chain ID checks [#1482](https://github.com/astriaorg/astria/pull/1482).

### Changed

- Replace `once_cell` with `LazyLock` [#1576](https://github.com/astriaorg/astria/pull/1576).
- Migrate all instances of `#[allow]` to `#[expect]` [#1561](https://github.com/astriaorg/astria/pull/1561).
- Code freeze through github actions [#1588](https://github.com/astriaorg/astria/pull/1588).
- Upgrade to proto `v1`s throughout [#1672](https://github.com/astriaorg/astria/pull/1672).

### Fixed

- Fix flaky restart test [#1575](https://github.com/astriaorg/astria/pull/1575).
- Remove enable mint entry from example env config [#1674](https://github.com/astriaorg/astria/pull/1674).

## [0.20.1] - 2024-09-06

### Changed

- Improve instrumentation [#1330](https://github.com/astriaorg/astria/pull/1330).

## [0.20.0] - 2024-08-22

### Changed

- Update `bytemark` dependency based on cargo audit warning [#1350](https://github.com/astriaorg/astria/pull/1350).
- Update with support for celestia-node v0.15.0 [#1367](https://github.com/astriaorg/astria/pull/1367).
- Support disabled celestia auth [#1372](https://github.com/astriaorg/astria/pull/1372).

## [0.19.0] - 2024-07-26

### Fixed

- Don't panic during panic [#1252](https://github.com/astriaorg/astria/pull/1252).
- Change execution API to use primitive RollupId [#1291](https://github.com/astriaorg/astria/pull/1291).

## [0.18.0] - 2024-06-27

### Added

- Add bech32m addresses [#1124](https://github.com/astriaorg/astria/pull/1124).

### Changed

- Remove non-bech32m address bytes [#1186](https://github.com/astriaorg/astria/pull/1186).

## [0.17.0] - 2024-06-04

### Added

- Rate limit sequencer cometbft requests [#1068](https://github.com/astriaorg/astria/pull/1068).
- Add retry to execution API gRPCs [#1115](https://github.com/astriaorg/astria/pull/1115).
- Add metrics [#1091](https://github.com/astriaorg/astria/pull/1091).

### Changed

- Perform signal handling in binary and test shutdown logic [#1094](https://github.com/astriaorg/astria/pull/1094).
- Celestia base heights in commitment state [#1121](https://github.com/astriaorg/astria/pull/1121).
- Skip outdated block metadata [#1120](https://github.com/astriaorg/astria/pull/1120).

## [0.16.0] - 2024-05-21

### Changed

- Update `SignedTransaction` to contain `Any` for transaction [#1044](https://github.com/astriaorg/astria/pull/1044).

### Fixed

- Don't exit on bad Sequencer connection [#1076](https://github.com/astriaorg/astria/pull/1076).
- Respect shutdown signals during init [#1080](https://github.com/astriaorg/astria/pull/1080).

## [0.15.0] - 2024-05-09

### Added

- Fetch missing blocks as necessary [#1054](https://github.com/astriaorg/astria/pull/1054).

### Changed

- Batch multiple Sequencer blocks to save on Celestia fees [#1045](https://github.com/astriaorg/astria/pull/1045).

### Fixed

- Only execute firm blocks if firm and soft block numbers match [#1021](https://github.com/astriaorg/astria/pull/1021).
- Retry blob fetch on request timeout [#1061](https://github.com/astriaorg/astria/pull/1061).

## [0.14.0] - 2024-04-26

### Added

- Create `sequencerblockapis` `v1alpha1` [#939](https://github.com/astriaorg/astria/pull/939).
- Add blackbox tests for conductor running in soft-only mode [#917](https://github.com/astriaorg/astria/pull/917).
- Brotli compress data blobs [#1006](https://github.com/astriaorg/astria/pull/1006).

### Changed

- Update `SequencerBlockHeader` and related proto types to not use cometbft
header [#830](https://github.com/astriaorg/astria/pull/830).
- Update execution service to use sequencerblock [#954](https://github.com/astriaorg/astria/pull/954).
- Fully split `sequencerapis` and remove [#958](https://github.com/astriaorg/astria/pull/958).
- Fetch blocks pending finalization [#980](https://github.com/astriaorg/astria/pull/980).

### Fixed

- Robust Celestia blob fetch, verify, convert [#946](https://github.com/astriaorg/astria/pull/946).

## [0.13.1] - 2024-04-05

### Added

- Add serialization to execution `v1alpha2` compliant with protobuf json
mapping [#857](https://github.com/astriaorg/astria/pull/857).

### Changed

- Simplify builder types by config-like [#829](https://github.com/astriaorg/astria/pull/829).
- Use cancellation tokens for shutdown [#845](https://github.com/astriaorg/astria/pull/845).
- Generate pbjon impls for sequencer types needed to mock conductor [#905](https://github.com/astriaorg/astria/pull/905).
- Replace hex by base64 for display formatting, emitting tracing events [#908](https://github.com/astriaorg/astria/pull/908).

### Fixed

- Bump otel to resolve panics in layered span access [#820](https://github.com/astriaorg/astria/pull/820).
- Don't panic while shutting down [#846](https://github.com/astriaorg/astria/pull/846).
- Clarify conductor log [#868](https://github.com/astriaorg/astria/pull/868).
- Enable tls for grpc connections [#925](https://github.com/astriaorg/astria/pull/925).

## [0.13.0] - 2024-03-19

### Added

- Provide explicit HTTP, Websocket Celestia RPC URLs [#780](https://github.com/astriaorg/astria/pull/780).
- Report if conductor won't read more Celestia heights [#799](https://github.com/astriaorg/astria/pull/799).

### Changed

- Simplify emitting error fields with cause chains [#765](https://github.com/astriaorg/astria/pull/765).
- Assert host fulfills execution API contract [#772](https://github.com/astriaorg/astria/pull/772).
- Update increment celestia height to fetch [#801](https://github.com/astriaorg/astria/pull/801).
- Use Celestia crates published on crates.io [#806](https://github.com/astriaorg/astria/pull/806).
- Emit more information about blocks received from Sequencer, Celestia [#811](https://github.com/astriaorg/astria/pull/811).
- Use Sequencer gRPC API to fetch soft bocks. [#815](https://github.com/astriaorg/astria/pull/815).
- Migrate `v1alpha1` sequencer apis to `v1` [#817](https://github.com/astriaorg/astria/pull/817).

### Removed

- Remove all optimism functionality [#775](https://github.com/astriaorg/astria/pull/775).
- Delete unused proto file [#783](https://github.com/astriaorg/astria/pull/783).

### Fixed

- Keep `wsclient` alive [#762](https://github.com/astriaorg/astria/pull/762).
- Simplify mapping Celestia heights to Sequencer heights [#797](https://github.com/astriaorg/astria/pull/797).
- Serialize rollup IDs as strings so telemetry doesn't crash [#821](https://github.com/astriaorg/astria/pull/821).

## [0.12.0] - 2024-02-15

### Added

- Add `SignedTransaction::sha256_of_proto_encoding()` method [#687](https://github.com/astriaorg/astria/pull/687).
- Add `ibc_sudo_address` to genesis, only allow `IbcRelay` actions from this
address [#721](https://github.com/astriaorg/astria/pull/721).
- Add firm block syncing [#691](https://github.com/astriaorg/astria/pull/691).
- Use opentelemetry [#656](https://github.com/astriaorg/astria/pull/656).
- Metrics setup [#739](https://github.com/astriaorg/astria/pull/739) and [#750](https://github.com/astriaorg/astria/pull/750).
- Add `ibc_relayer_addresses` list and allow modifications via
`ibc_sudo_address` [#737](https://github.com/astriaorg/astria/pull/737).
- Add pretty-printing to stdout [#736](https://github.com/astriaorg/astria/pull/736).
- Print build info in all services [#753](https://github.com/astriaorg/astria/pull/753).

### Changed

- Transfer fees to block proposer instead of burning [#690](https://github.com/astriaorg/astria/pull/690).
- Update licenses [#706](https://github.com/astriaorg/astria/pull/706).
- Move fee asset from `UnsignedTransaction` to `SequenceAction` and
`TransferAction` [#719](https://github.com/astriaorg/astria/pull/719).
- Build all binaries, fix pr title ci [#728](https://github.com/astriaorg/astria/pull/728).
- Split protos into multiple buf repos [#732](https://github.com/astriaorg/astria/pull/732).
- Bump rust to 1.76, cargo-chef to 0.1.63 [#744](https://github.com/astriaorg/astria/pull/744).
- Aet permitted commitment spread from rollup genesis [#743](https://github.com/astriaorg/astria/pull/743).

### Fixed

- Fix `FungibleTokenPacketData` decoding [#686](https://github.com/astriaorg/astria/pull/686).
- Relax size requirements of hash buffers [#709](https://github.com/astriaorg/astria/pull/709).
- Replace allocating display impl [#738](https://github.com/astriaorg/astria/pull/738).
- Fix docker builds [#756](https://github.com/astriaorg/astria/pull/756).

## [0.11.1] - 2024-01-10

### Added

- Lint debug fields in tracing events [#664](https://github.com/astriaorg/astria/pull/664).

### Changed

- Use methods to increment, ensure macro to compare [#619](https://github.com/astriaorg/astria/pull/619).
- Add proto formatting, cleanup justfile [#637](https://github.com/astriaorg/astria/pull/637).
- Bump all checkout actions in CI to v3 [#641](https://github.com/astriaorg/astria/pull/641).
- Unify construction of cometbft blocks in tests [#640](https://github.com/astriaorg/astria/pull/640).
- Switch tagging format in CI [#639](https://github.com/astriaorg/astria/pull/639).
- Rename astria-proto to astria-core [#644](https://github.com/astriaorg/astria/pull/644).
- Break up sequencer `v1alpha1` module [#646](https://github.com/astriaorg/astria/pull/646).
- Don't deny unknown config fields [#657](https://github.com/astriaorg/astria/pull/657).
- Define abci error codes in protobuf [#647](https://github.com/astriaorg/astria/pull/647).
- Use display formatting instead of debug formatting in tracing events [#671](https://github.com/astriaorg/astria/pull/671).

### Fixed

- Add error context, simplify type conversions [#620](https://github.com/astriaorg/astria/pull/620).
- Fail hard when executing blocks fails [#621](https://github.com/astriaorg/astria/pull/621).
- Amend Cargo.toml when building images [#672](https://github.com/astriaorg/astria/pull/672).

## [0.11.0] - 2023-11-30

### Added

- Don't attempt to execute finalized blocks [#617](https://github.com/astriaorg/astria/pull/617).

### Changed

- Make block verifier a submodule of data availability [#593](https://github.com/astriaorg/astria/pull/593).
- Require chain_id be 32 bytes [#436](https://github.com/astriaorg/astria/pull/436).
- Redefine sequencer blocks, celestia blobs as protobuf [#395](https://github.com/astriaorg/astria/pull/395).

### Fixed

- Validator height should be trailing [#613](https://github.com/astriaorg/astria/pull/613).

## [0.10.0] - 2023-11-18

### Added

- Add an RFC 6962 compliant Merkle tree with flat memory representation [#554](https://github.com/astriaorg/astria/pull/554).
- Implement derivation and execution of optimism `DepositTransaction`s [#535](https://github.com/astriaorg/astria/pull/535).

## [0.9.0] - 2023-11-14

### Changed

- Implement clippy pedantic suggestions [#573](https://github.com/astriaorg/astria/pull/573).

### Fixed

- Use sequencer chain id for sequencer blobs [#577](https://github.com/astriaorg/astria/pull/577).

## [0.8.0] - 2023-11-07

### Added

- Add re-sync for missed sequencer blocks [#515](https://github.com/astriaorg/astria/pull/515).
- Add commitment grab to better set sync start height [#553](https://github.com/astriaorg/astria/pull/553).

### Changed

- Celestia-client: use eiger's version [#486](https://github.com/astriaorg/astria/pull/486).
- Replace formatted error backtraces by value impl [#516](https://github.com/astriaorg/astria/pull/516).
- `v1alpha2` integration [#528](https://github.com/astriaorg/astria/pull/528).
- Define service configs in terms of a central crate [#537](https://github.com/astriaorg/astria/pull/537).
- Verify current block commit in conductor; remove `last_commit` from
`SequencerBlockData` [#560](https://github.com/astriaorg/astria/pull/560).

### Removed

- Remove signing and signature verification of data posted to DA [#538](https://github.com/astriaorg/astria/pull/538).
- Remove disable empty block execution config setting [#556](https://github.com/astriaorg/astria/pull/556).

### Fixed

- Update rollup chain id in conductor example env to match composer [#505](https://github.com/astriaorg/astria/pull/505).
- Clarify logging in executor [#508](https://github.com/astriaorg/astria/pull/508).
- Implement `chain_ids_commitment` inclusion proof generation and verification [#548](https://github.com/astriaorg/astria/pull/548).
- Empty blocks from da get executed [#551](https://github.com/astriaorg/astria/pull/551).
- Dependency update for yanked `ahash` deps [#544](https://github.com/astriaorg/astria/pull/544).

## [0.7.0] - 2023-10-13

### Added

- Add execution commit level v2 [#474](https://github.com/astriaorg/astria/pull/474).
- Report cause of failed nonce fetch [#492](https://github.com/astriaorg/astria/pull/492).

### Changed

- Use fork of tendermint with backported `reqwest` client [#498](https://github.com/astriaorg/astria/pull/498).
- Never recycle websocket clients [#499](https://github.com/astriaorg/astria/pull/499).
- Spawn driver as task and report exit [#500](https://github.com/astriaorg/astria/pull/500).
- Resubscribe with backoff instead of failing [#501](https://github.com/astriaorg/astria/pull/501).

## [0.6.1] - 2023-10-12

### Added

- Log task exit [#479](https://github.com/astriaorg/astria/pull/479).

### Changed

- Bump penumbra, tendermint; prune workspace cargo of unused deps [#468](https://github.com/astriaorg/astria/pull/468).
- Reconnect to sequencer websocket with backoff [#483](https://github.com/astriaorg/astria/pull/483).

### Fixed

- Don't panic on empty blocks [#467](https://github.com/astriaorg/astria/pull/467).
- Fix action tree root inclusion proof verification [#469](https://github.com/astriaorg/astria/pull/469).

## [0.6.0] - 2023-10-05

### Changed

- Add genesis sequencer block height to config and env vars [#445](https://github.com/astriaorg/astria/pull/445).
- Refactor and implement full node sync from sequencer [#455](https://github.com/astriaorg/astria/pull/455).

## [0.5.1] - 2023-09-27

### Fixed

- Bug fixes related to validating data and allowing empty rollup blocks.
- Fix tendermint block to `SequencerBlockData` conversion [#424](https://github.com/astriaorg/astria/pull/424).
- Continue to execution when block subset empty [#426](https://github.com/astriaorg/astria/pull/426).

## 0.5.0 - 2023-09-22

### Added

- Initial release.

[unreleased]: https://github.com/astriaorg/astria/compare/conductor-v2.0.0-rc.2...HEAD
[2.0.0-rc.2]: https://github.com/astriaorg/astria/compare/conductor-v2.0.0-rc.1...conductor-v2.0.0-rc.2
[2.0.0-rc.1]: https://github.com/astriaorg/astria/compare/conductor-v1.1.0...conductor-v2.0.0-rc.1
[1.1.0]: https://github.com/astriaorg/astria/compare/conductor-v1.0.0...conductor-v1.1.0
[1.0.0]: https://github.com/astriaorg/astria/compare/conductor-v1.0.0-rc.2...conductor-v1.0.0
[1.0.0-rc.2]: https://github.com/astriaorg/astria/compare/conductor-v1.0.0-rc.1...conductor-v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/astriaorg/astria/compare/conductor-v0.20.1...conductor-v1.0.0-rc.1
[0.20.1]: https://github.com/astriaorg/astria/compare/conductor-v0.20.0...conductor-v0.20.1
[0.20.0]: https://github.com/astriaorg/astria/compare/conductor-v0.19.0...conductor-v0.20.0
[0.19.0]: https://github.com/astriaorg/astria/compare/conductor-v0.18.0...conductor-v0.19.0
[0.18.0]: https://github.com/astriaorg/astria/compare/conductor-v0.17.0...conductor-v0.18.0
[0.17.0]: https://github.com/astriaorg/astria/compare/conductor-v0.16.0...conductor-v0.17.0
[0.16.0]: https://github.com/astriaorg/astria/compare/conductor-v0.15.0...conductor-v0.16.0
[0.15.0]: https://github.com/astriaorg/astria/compare/conductor-v0.14.0...conductor-v0.15.0
[0.14.0]: https://github.com/astriaorg/astria/compare/conductor-v0.13.1...conductor-v0.14.0
[0.13.1]: https://github.com/astriaorg/astria/compare/conductor-v0.13.0...conductor-v0.13.1
[0.13.0]: https://github.com/astriaorg/astria/compare/conductor-v0.12.0...conductor-v0.13.0
[0.12.0]: https://github.com/astriaorg/astria/compare/conductor-v0.11.1...conductor-v0.12.0
[0.11.1]: https://github.com/astriaorg/astria/compare/conductor-v0.11.0...conductor-v0.11.1
[0.11.0]: https://github.com/astriaorg/astria/compare/v0.10.1--conductor...v0.11.0--conductor
[0.10.0]: https://github.com/astriaorg/astria/compare/v0.9.0--conductor...v0.10.0--conductor
[0.9.0]: https://github.com/astriaorg/astria/compare/v0.8.0--conductor...v0.9.0--conductor
[0.8.0]: https://github.com/astriaorg/astria/compare/v0.7.0--conductor...v0.8.0--conductor
[0.7.0]: https://github.com/astriaorg/astria/compare/v0.6.1--conductor...v0.7.0--conductor
[0.6.1]: https://github.com/astriaorg/astria/compare/v0.6.0--conductor...v0.6.1--conductor
[0.6.0]: https://github.com/astriaorg/astria/compare/v0.5.1--conductor...v0.6.0--conductor
[0.5.1]: https://github.com/astriaorg/astria/compare/v0.5.0--conductor...v0.5.1--conductor
