<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## Changed

- Index all send packet event attributes [#1786](https://github.com/astriaorg/astria/pull/1786).

## [1.0.0] - 2024-10-25

### Changed

- Bump penumbra dependencies [#1740](https://github.com/astriaorg/astria/pull/1740).
- Move fee event recording to transaction from block [#1718](https://github.com/astriaorg/astria/pull/1718).

## [1.0.0-rc.2] - 2024-10-23

### Changed

- Make ABCI response for account balances deterministic [#1574](https://github.com/astriaorg/astria/pull/1574).
- Move and improve transaction fee estimation [#1722](https://github.com/astriaorg/astria/pull/1722).
- Make fees optional at genesis [#1664](https://github.com/astriaorg/astria/pull/1664).
- Add test for rollup refund in [#1728](https://github.com/astriaorg/astria/pull/1728).
- Make native asset optional [#1703](https://github.com/astriaorg/astria/pull/1703).

### Removed

- Remove unused asset storage variant [#1704](https://github.com/astriaorg/astria/pull/1704).

### Fixed

- Fix fee estimation [#1701](https://github.com/astriaorg/astria/pull/1701).

## [1.0.0-rc.1] - 2024-10-17

### Added

- Add traceability to rollup deposits [#1410](https://github.com/astriaorg/astria/pull/1410).
- Report deposit events [#1447](https://github.com/astriaorg/astria/pull/1447).
- Add IBC sudo change action [#1509](https://github.com/astriaorg/astria/pull/1509).
- Transaction categories on `UnsignedTransaction` [#1512](https://github.com/astriaorg/astria/pull/1512).
- Provide astrotrek chart [#1513](https://github.com/astriaorg/astria/pull/1513).

### Changed

- Change test addresses to versions with known private keys [#1487](https://github.com/astriaorg/astria/pull/1487).
- Make mempool balance aware [#1408](https://github.com/astriaorg/astria/pull/1408).
- Migrate from `anyhow::Result` to `eyre::Result` [#1387](https://github.com/astriaorg/astria/pull/1387).
- Change `Deposit` byte length calculation [#1507](https://github.com/astriaorg/astria/pull/1507).
- Put blocks and deposits to non-verified storage (ENG-812) [#1525](https://github.com/astriaorg/astria/pull/1525).
- Replace `once_cell` with `LazyLock` [#1576](https://github.com/astriaorg/astria/pull/1576).
- Use builder pattern for transaction container tests [#1592](https://github.com/astriaorg/astria/pull/1592).
- Exclusively use Borsh encoding for stored data [#1492](https://github.com/astriaorg/astria/pull/1492).
- Genesis chart template to support latest changes [#1594](https://github.com/astriaorg/astria/pull/1594).
- Simplify boolean expressions in `transaction container` [#1595](https://github.com/astriaorg/astria/pull/1595).
- Make empty transactions invalid  [#1609](https://github.com/astriaorg/astria/pull/1609).
- Rewrite `check_tx` to be more efficient and fix regression [#1515](https://github.com/astriaorg/astria/pull/1515).
- Generate `SequencerBlock` after transaction execution in proposal phase [#1562](https://github.com/astriaorg/astria/pull/1562).
- Add limit to total amount of transactions in parked  [#1638](https://github.com/astriaorg/astria/pull/1638).
- Remove action suffix from all action types [#1630](https://github.com/astriaorg/astria/pull/1630).
- Update `futures-util` dependency based on cargo audit warning [#1644](https://github.com/astriaorg/astria/pull/1644).
- Update storage keys locations and values (ENG-898) [#1616](https://github.com/astriaorg/astria/pull/1616).
- Enforce block ordering by transaction group  [#1618](https://github.com/astriaorg/astria/pull/1618).
- Rework all fees [#1647](https://github.com/astriaorg/astria/pull/1647).
- Prefer `astria.primitive.v1.RollupId` over bytes [#1661](https://github.com/astriaorg/astria/pull/1661).
- Call transactions `Transaction`, contents `TransactionBody` [#1650](https://github.com/astriaorg/astria/pull/1650).
- Rename sequence action to rollup data submission [#1665](https://github.com/astriaorg/astria/pull/1665).
- Upgrade to proto `v1`s throughout [#1672](https://github.com/astriaorg/astria/pull/1672).

### Removed

- Remove unused enable mint env [#1673](https://github.com/astriaorg/astria/pull/1673).

### Fixed

- Add `end_block` to `app_execute_transaction_with_every_action_snapshot` [#1455](https://github.com/astriaorg/astria/pull/1455).
- Fix incorrect error message from `BridgeUnlock` actions [#1505](https://github.com/astriaorg/astria/pull/1505).
- Fix and refactor ics20 logic [#1495](https://github.com/astriaorg/astria/pull/1495).
- Install astria-eyre hook [#1552](https://github.com/astriaorg/astria/pull/1552).
- Provide context in `check_tx` response log [#1506](https://github.com/astriaorg/astria/pull/1506).
- Fix app hash in horcrux sentries [#1646](https://github.com/astriaorg/astria/pull/1646).
- Allow compat prefixed addresses when receiving ics20 transfers [#1655](https://github.com/astriaorg/astria/pull/1655).
- Remove enable mint entry from example env config [#1674](https://github.com/astriaorg/astria/pull/1674).

## [0.17.0] - 2024-09-06

### Changed

- BREAKING: Enforce withdrawals consumed [#1391](https://github.com/astriaorg/astria/pull/1391).
- BREAKING: Permit bech32 compatible addresses [#1425](https://github.com/astriaorg/astria/pull/1425).
- Memoize `address_bytes` of verification key [#1444](https://github.com/astriaorg/astria/pull/1444).

## [0.16.0] - 2024-08-22

### Added

- Add fee reporting [#1305](https://github.com/astriaorg/astria/pull/1305).

### Changed

- Update `bytemark` dependency based on cargo audit warning [#1350](https://github.com/astriaorg/astria/pull/1350).
- BREAKING: Take funds from bridge in ics20 withdrawals [#1344](https://github.com/astriaorg/astria/pull/1344).
- BREAKING: Require that bridge unlock address always be set [#1339](https://github.com/astriaorg/astria/pull/1339).
- Rewrite mempool to have per-account transaction storage and maintenance  [#1323](https://github.com/astriaorg/astria/pull/1323).

### Removed

- Remove global state [#1317](https://github.com/astriaorg/astria/pull/1317).

### Fixed

- Fix abci error code [#1280](https://github.com/astriaorg/astria/pull/1280).
- BREAKING: Fix TOCTOU issues by merging check and execution [#1332](https://github.com/astriaorg/astria/pull/1332).
- Fix block fee collection [#1343](https://github.com/astriaorg/astria/pull/1343).
- Bump penumbra dep to fix ibc state access bug [#1389](https://github.com/astriaorg/astria/pull/1389).

## [0.15.0] - 2024-07-26

### Added

- Implement transaction fee query [#1196](https://github.com/astriaorg/astria/pull/1196).
- Add metrics [#1248](https://github.com/astriaorg/astria/pull/1248).
- Add mempool benchmarks [#1238](https://github.com/astriaorg/astria/pull/1238).

### Changed

- Generate serde traits impls for all protocol protobufs [#1260](https://github.com/astriaorg/astria/pull/1260).
- Define bridge memos in proto [#1285](https://github.com/astriaorg/astria/pull/1285).

### Fixed

- Fix prepare proposal metrics [#1211](https://github.com/astriaorg/astria/pull/1211).
- Fix wrong metric and remove unused metric [#1240](https://github.com/astriaorg/astria/pull/1240).
- Store native asset ibc->trace mapping in `init_chain` [#1242](https://github.com/astriaorg/astria/pull/1242).
- Disambiguate return addresses [#1266](https://github.com/astriaorg/astria/pull/1266).
- Improve and fix instrumentation [#1255](https://github.com/astriaorg/astria/pull/1255).

## [0.14.1] - 2024-07-03

### Added

- Implement abci query for bridge account info [#1189](https://github.com/astriaorg/astria/pull/1189).

### Fixed

- Update asset query path [#1141](https://github.com/astriaorg/astria/pull/1141).

## [0.14.0] - 2024-06-27

### Added

- Add `allowed_fee_asset_ids` abci query and `sequencer_client` support [#1127](https://github.com/astriaorg/astria/pull/1127).
- Implement `bridge/account_last_tx_hash` abci query [#1158](https://github.com/astriaorg/astria/pull/1158).
- Add bech32m addresses [#1124](https://github.com/astriaorg/astria/pull/1124).
- Implement refund to rollup logic upon ics20 transfer refund [#1161](https://github.com/astriaorg/astria/pull/1161).
- Implement bridge sudo and withdrawer addresses [#1142](https://github.com/astriaorg/astria/pull/1142).
- Add ttl and invalid cache to app mempool [#1138](https://github.com/astriaorg/astria/pull/1138).
- Implement `Ics20TransferDepositMemo` format for incoming ics20 transfers to
bridge accounts [#1202](https://github.com/astriaorg/astria/pull/1202).
- Add ibc memo type snapshot tests [#1205](https://github.com/astriaorg/astria/pull/1205).
- Allow configuring base address prefix [#1201](https://github.com/astriaorg/astria/pull/1201).

### Changed

- Query full denomination from asset ID [#1067](https://github.com/astriaorg/astria/pull/1067).
- Add `clippy::arithmetic-side-effects` lint and fix resulting warnings [#1081](https://github.com/astriaorg/astria/pull/1081).
- Use macro to declare metric constants [#1129](https://github.com/astriaorg/astria/pull/1129).
- Bump penumbra deps [#1159](https://github.com/astriaorg/astria/pull/1159).
- Register all metrics during startup [#1144](https://github.com/astriaorg/astria/pull/1144).
- Parse ics20 denoms as ibc or trace prefixed variants [#1181](https://github.com/astriaorg/astria/pull/1181).
- Remove non-bech32m address bytes [#1186](https://github.com/astriaorg/astria/pull/1186).
- Bump penumbra deps [#1216](https://github.com/astriaorg/astria/pull/1216).
- Use full IBC ICS20 denoms instead of IDs [#1209](https://github.com/astriaorg/astria/pull/1209).

### Removed

- Remove mint module [#1134](https://github.com/astriaorg/astria/pull/1134).

### Fixed

- Prefix removal source non-refund ics20 packet [#1162](https://github.com/astriaorg/astria/pull/1162).

## [0.13.0] - 2024-05-23

### Added

- Implement `get_pending_nonce` for sequencer API [#1073](https://github.com/astriaorg/astria/pull/1073).

### Changed

- Fees go to sudo poa [#1104](https://github.com/astriaorg/astria/pull/1104).

## [0.12.0] - 2024-05-21

### Added

- Implement basic app side mempool with nonce ordering [#1000](https://github.com/astriaorg/astria/pull/1000).
- Add fees to genesis state [#1055](https://github.com/astriaorg/astria/pull/1055).
- Implement bridge unlock action and derestrict transfers [#1034](https://github.com/astriaorg/astria/pull/1034).
- Implement `FeeChangeAction` for the authority component [#1037](https://github.com/astriaorg/astria/pull/1037).

### Changed

- Store fees for actions in app state [#1017](https://github.com/astriaorg/astria/pull/1017).
- Update ics20 withdrawal to have a memo field [#1056](https://github.com/astriaorg/astria/pull/1056).
- Update `SignedTransaction` to contain `Any` for transaction [#1044](https://github.com/astriaorg/astria/pull/1044).

### Fixed

- Stateful check now ensures balance for total tx [#1009](https://github.com/astriaorg/astria/pull/1009).
- Set current app hash properly when creating app [#1025](https://github.com/astriaorg/astria/pull/1025).
- Panic sequencer instead of cometbft on erroring abci consensus requests [#1016](https://github.com/astriaorg/astria/pull/1016).
- Fix ibc prefix conversion [#1065](https://github.com/astriaorg/astria/pull/1065).

## [0.11.0] - 2024-04-26

### Added

- Add cargo audit to CI [#887](https://github.com/astriaorg/astria/pull/887).
- Add unit tests for state extension trait [#890](https://github.com/astriaorg/astria/pull/890).
- Create `sequencerblockapis` `v1alpha1` [#939](https://github.com/astriaorg/astria/pull/939).
- Add display for deposits in `end_block` [#864](https://github.com/astriaorg/astria/pull/864).
- Create wrapper types for `RollupId` and `Account` [#987](https://github.com/astriaorg/astria/pull/987).
- Add initial set of metrics to sequencer [#965](https://github.com/astriaorg/astria/pull/965).

### Changed

- Check for sufficient balance in `check_tx` [#869](https://github.com/astriaorg/astria/pull/869).
- Generate names for protobuf rust types [#904](https://github.com/astriaorg/astria/pull/904).
- Replace hex by base64 for display formatting, emitting tracing events [#908](https://github.com/astriaorg/astria/pull/908).
- Set revision number from chain id in `init_chain` [#935](https://github.com/astriaorg/astria/pull/935).
- Update `SequencerBlockHeader` and related proto types to not use cometbft
header [#830](https://github.com/astriaorg/astria/pull/830).
- Update to ABCI v0.38 [#831](https://github.com/astriaorg/astria/pull/831).
- Fully split `sequencerapis` and remove [#958](https://github.com/astriaorg/astria/pull/958).
- Require chain id in transactions [#973](https://github.com/astriaorg/astria/pull/973).
- Update justfile and testnet script [#985](https://github.com/astriaorg/astria/pull/985).
- Bridge account only takes a single asset [#988](https://github.com/astriaorg/astria/pull/988).

### Removed

- No telemetry for formatting db keys [#909](https://github.com/astriaorg/astria/pull/909).
- Remove `SequencerBlock::try_from_cometbft` [#1005](https://github.com/astriaorg/astria/pull/1005).

### Fixed

- Make `get_deposit_rollup_ids` not return duplicates [#916](https://github.com/astriaorg/astria/pull/916).
- `is_proposer` check now considers proposer's address [#936](https://github.com/astriaorg/astria/pull/936).
- Respect `max_tx_bytes` when preparing proposals [#911](https://github.com/astriaorg/astria/pull/911).
- Fix state setup to be consistent before transaction execution [#945](https://github.com/astriaorg/astria/pull/945).
- Don't store execution result of failed tx [#992](https://github.com/astriaorg/astria/pull/992).
- Don't allow sudo to cause consensus failures [#999](https://github.com/astriaorg/astria/pull/999).

## [0.10.1] - 2024-04-03

### Added

- Implement bridge deposits for incoming ICS20 transfers [#843](https://github.com/astriaorg/astria/pull/843).
- Add serialization to execution `v1alpha2` compliant with protobuf json
mapping [#857](https://github.com/astriaorg/astria/pull/857).
- Add unit tests for state extension traits
[#858](https://github.com/astriaorg/astria/pull/858),
[#871](https://github.com/astriaorg/astria/pull/871),
[#874](https://github.com/astriaorg/astria/pull/874),
[#875](https://github.com/astriaorg/astria/pull/875),
[#876](https://github.com/astriaorg/astria/pull/876) and
[#878](https://github.com/astriaorg/astria/pull/878).

### Changed

- Use `Arc<Self>` target in generated gRPC service traits [#853](https://github.com/astriaorg/astria/pull/853).
- Logging as human readable for account state [#898](https://github.com/astriaorg/astria/pull/898).

### Fixed

- Bump otel to resolve panics in layered span access [#820](https://github.com/astriaorg/astria/pull/820).
- Fix `is_source` prefix check [#844](https://github.com/astriaorg/astria/pull/844).
- Fix escrow channel check when receiving non-refund ics20 packet [#851](https://github.com/astriaorg/astria/pull/851).
- Fix rollup ids commitment for deposits [#863](https://github.com/astriaorg/astria/pull/863).

## [0.10.0] - 2024-03-19

### Added

- Add sequencer service proto [#701](https://github.com/astriaorg/astria/pull/701).
- Implement bridge accounts and related actions [#768](https://github.com/astriaorg/astria/pull/768).

### Changed

- Simplify emitting error fields with cause chains [#765](https://github.com/astriaorg/astria/pull/765).
- Update dependencies [#782](https://github.com/astriaorg/astria/pull/782).
- Store sequencer blocks in the sequencer state [#787](https://github.com/astriaorg/astria/pull/787).
- Include deposit data as part of rollup data [#802](https://github.com/astriaorg/astria/pull/802).
- Bump penumbra deps [#825](https://github.com/astriaorg/astria/pull/825).

### Fixed

- Filtered blocks success when no data expected [#819](https://github.com/astriaorg/astria/pull/819).
- Fix bug in `get_sequencer_block_by_hash` [#832](https://github.com/astriaorg/astria/pull/832).

## [0.9.0] - 2024-02-15

### Added

- Add `SignedTransaction::sha256_of_proto_encoding()` method [#687](https://github.com/astriaorg/astria/pull/687).
- Add `ibc_sudo_address` to genesis, only allow `IbcRelay` actions from this
address [#721](https://github.com/astriaorg/astria/pull/721).
- Use opentelemetry [#656](https://github.com/astriaorg/astria/pull/656).
- Allow specific assets for fee payment [#730](https://github.com/astriaorg/astria/pull/730).
- Metrics setup [#739](https://github.com/astriaorg/astria/pull/739) and [#750](https://github.com/astriaorg/astria/pull/750).
- Add `ibc_relayer_addresses` list and allow modifications via
`ibc_sudo_address` [#737](https://github.com/astriaorg/astria/pull/737).
- Add pretty-printing to stdout [#736](https://github.com/astriaorg/astria/pull/736).
- Implement ability to update fee assets using sudo key [#752](https://github.com/astriaorg/astria/pull/752).
- Print build info in all services [#753](https://github.com/astriaorg/astria/pull/753).

### Changed

- Transfer fees to block proposer instead of burning [#690](https://github.com/astriaorg/astria/pull/690).
- Update licenses [#706](https://github.com/astriaorg/astria/pull/706).
- Update balance queries to return every asset owned by account [#683](https://github.com/astriaorg/astria/pull/683).
- Use `IbcComponent` and penumbra `HostInterface` [#700](https://github.com/astriaorg/astria/pull/700).
- Move fee asset from `UnsignedTransaction` to `SequenceAction` and
`TransferAction` [#719](https://github.com/astriaorg/astria/pull/719).
- Relax size requirements of hash buffers [#709](https://github.com/astriaorg/astria/pull/709).
- Split protos into multiple buf repos [#732](https://github.com/astriaorg/astria/pull/732).
- Add fee for `Ics20Withdrawal` action [#733](https://github.com/astriaorg/astria/pull/733).
- Bump rust to 1.76, cargo-chef to 0.1.63 [#744](https://github.com/astriaorg/astria/pull/744).
- Upgrade to penumbra release 0.66 [#741](https://github.com/astriaorg/astria/pull/741).
- Move ibc-related code to its own module [#757](https://github.com/astriaorg/astria/pull/757).

### Fixed

- Fix `FungibleTokenPacketData` decoding [#686](https://github.com/astriaorg/astria/pull/686).
- Replace allocating display impl [#738](https://github.com/astriaorg/astria/pull/738).
- Fix docker builds [#756](https://github.com/astriaorg/astria/pull/756).

## [0.8.0] - 2024-01-10

### Added

- Add proto formatting, cleanup justfile [#637](https://github.com/astriaorg/astria/pull/637).
- Implement ICS20 withdrawals [#609](https://github.com/astriaorg/astria/pull/609).
- Add IBC gRPC server to sequencer app [#631](https://github.com/astriaorg/astria/pull/631).
- Lint debug fields in tracing events [#664](https://github.com/astriaorg/astria/pull/664).

### Changed

- Move protobuf specs to repository top level [#629](https://github.com/astriaorg/astria/pull/629).
- Bump all checkout actions in CI to v3 [#641](https://github.com/astriaorg/astria/pull/641).
- Unify construction of cometbft blocks in tests [#640](https://github.com/astriaorg/astria/pull/640).
- Store mapping of IBC asset ID to full denomination trace [#614](https://github.com/astriaorg/astria/pull/614).
- Switch tagging format in CI [#639](https://github.com/astriaorg/astria/pull/639).
- Bump penumbra deps [#655](https://github.com/astriaorg/astria/pull/655).
- Rename `astria-proto` to `astria-core` [#644](https://github.com/astriaorg/astria/pull/644).
- Break up `v1alpha1` module [#646](https://github.com/astriaorg/astria/pull/646).
- Don't deny unknown config fields [#657](https://github.com/astriaorg/astria/pull/657).
- Call abort on ABCI server on signal [#670](https://github.com/astriaorg/astria/pull/670).
- Define abci error codes in protobuf [#647](https://github.com/astriaorg/astria/pull/647).
- Use display formatting instead of debug formatting in tracing events [#671](https://github.com/astriaorg/astria/pull/671).
- Update instrumentation for all consensus & app functions [#677](https://github.com/astriaorg/astria/pull/677).
- Add max sequencer bytes per block limit [#676](https://github.com/astriaorg/astria/pull/676).

### Removed

- Remove `AppHash` [#655](https://github.com/astriaorg/astria/pull/655).

### Fixed

- Adjust input to proto breaking change linter after refactor [#635](https://github.com/astriaorg/astria/pull/635).
- Fix ABCI event handling [#666](https://github.com/astriaorg/astria/pull/666).
- Clear processed tx count in `begin_block` [#659](https://github.com/astriaorg/astria/pull/659).
- Amend Cargo.toml when building images [#672](https://github.com/astriaorg/astria/pull/672).
- Update app state to latest committed before starting round [#673](https://github.com/astriaorg/astria/pull/673).
- Allow blocksync to complete successfully [#675](https://github.com/astriaorg/astria/pull/675).

## [0.7.0] - 2023-11-30

### Added

- Implement support for arbitrary assets [#568](https://github.com/astriaorg/astria/pull/568).
- Support `IbcAction`s and implement ICS20 incoming transfer application logic [#579](https://github.com/astriaorg/astria/pull/579).

### Changed

- Replace `buf-generate` by `tonic_build` [#581](https://github.com/astriaorg/astria/pull/581).
- Bump all dependencies (mainly penumbra, celestia, tendermint) [#582](https://github.com/astriaorg/astria/pull/582).
- Enforce sequencer blob invariants [#576](https://github.com/astriaorg/astria/pull/576).
- Require `chain_id` be 32 bytes [#436](https://github.com/astriaorg/astria/pull/436).
- Update penumbra-ibc features [#615](https://github.com/astriaorg/astria/pull/615).

### Fixed

- Fix instrument logging not to log every tx [#595](https://github.com/astriaorg/astria/pull/595).
- Cap tx size at 250kB [#601](https://github.com/astriaorg/astria/pull/601).

## [0.6.0] - 2023-11-18

### Added

- Add an RFC-6962 compliant Merkle tree with flat memory representation [#554](https://github.com/astriaorg/astria/pull/554).

## [0.5.0] - 2023-11-07

### Added

- Implement sudo key changes [#431](https://github.com/astriaorg/astria/pull/431).
- Implement minting module [#435](https://github.com/astriaorg/astria/pull/435).

### Changed

- Remove byzantine validators in `begin_block` [#429](https://github.com/astriaorg/astria/pull/429).
- Bump penumbra, tendermint; prune workspace cargo of unused deps [#468](https://github.com/astriaorg/astria/pull/468).
- Bump rust to 1.72 in CI [#477](https://github.com/astriaorg/astria/pull/477).
- Use fork of tendermint with backported `reqwest` client [#498](https://github.com/astriaorg/astria/pull/498).
- Move transaction execution to prepare/process proposal [#480](https://github.com/astriaorg/astria/pull/480).

### Fixed

- Fix tests without `--all-features` [#481](https://github.com/astriaorg/astria/pull/481).
- Fix typos [#541](https://github.com/astriaorg/astria/pull/541).
- Implement `chain_ids_commitment` inclusion proof generation and verification [#548](https://github.com/astriaorg/astria/pull/548).
- Fix authority component `ValidatorSet` non determinism [#557](https://github.com/astriaorg/astria/pull/557).
- Run only `prepare_proposal` if proposer [#558](https://github.com/astriaorg/astria/pull/558).

## [0.4.1] - 2023-09-27

### Added

- Implement basic validator set updates [#359](https://github.com/astriaorg/astria/pull/359).

### Fixed

- Fix mempool nonce check [#434](https://github.com/astriaorg/astria/pull/434).

## 0.4.0 - 2023-09-22

### Added

- Initial release.

[unreleased]: https://github.com/astriaorg/astria/compare/sequencer-v1.0.0...HEAD
[1.0.0]: https://github.com/astriaorg/astria/compare/sequencer-v1.0.0-rc.2...sequencer-v1.0.0
[1.0.0-rc.2]: https://github.com/astriaorg/astria/compare/sequencer-v1.0.0-rc.1...sequencer-v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/astriaorg/astria/compare/sequencer-v0.17.0...sequencer-v1.0.0-rc.1
[0.17.0]: https://github.com/astriaorg/astria/compare/cli-v0.4.0...sequencer-v0.17.0
[0.16.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.15.0...sequencer-v0.16.0
[0.15.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.14.1...sequencer-v0.15.0
[0.14.1]: https://github.com/astriaorg/astria/compare/sequencer-v0.14.0...sequencer-v0.14.1
[0.14.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.13.0...sequencer-v0.14.0
[0.13.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.12.0...sequencer-v0.13.0
[0.12.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.11.0...sequencer-v0.12.0
[0.11.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.10.1...sequencer-v0.11.0
[0.10.1]: https://github.com/astriaorg/astria/compare/sequencer-v0.10.0...sequencer-v0.10.1
[0.10.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.9.0...sequencer-v0.10.0
[0.9.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.8.0...sequencer-v0.9.0
[0.8.0]: https://github.com/astriaorg/astria/compare/sequencer-v0.7.0...sequencer-v0.8.0
[0.7.0]: https://github.com/astriaorg/astria/compare/v0.6.0--sequencer...v0.7.0--sequencer
[0.6.0]: https://github.com/astriaorg/astria/compare/v0.5.0--sequencer...v0.6.0--sequencer
[0.5.0]: https://github.com/astriaorg/astria/compare/v0.4.1--sequencer...v0.5.0--sequencer
[0.4.1]: https://github.com/astriaorg/astria/compare/v0.4.0--sequencer...v0.4.1--sequencer
