<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Added

- Initial release.
- Added method `TracePrefixed::leading_channel` to read the left-most channel of
  a trace prefixed ICS20 asset [#1768](https://github.com/astriaorg/astria/pull/1768)
- Added `impl Protobuf for Address<Bech32m>` [#1802](https://github.com/astriaorg/astria/pull/1802)

### Changed

- Moved `astria_core::crypto` to `astria-core-crypto` and reexported
  `astria_core_crypto as crypto` (this change is transparent)
  [#1800](https://github.com/astriaorg/astria/pull/1800/)
- Moved definitions of address domain type to `astria-core-address` and
  reexported items using the same aliases [#1802](https://github.com/astriaorg/astria/pull/1802)

### Removed

- Removed method `TracePrefixed::last_channel` [#1768](https://github.com/astriaorg/astria/pull/1768)
- Removed method `SigningKey::try_address` [#1800](https://github.com/astriaorg/astria/pull/1800/)
- Removed inherent methods `Address::try_from_raw` and `Address::to_raw`
  [#1802](https://github.com/astriaorg/astria/pull/1802)
- Removed `AddressBuilder::with_iter` from public interface [#1802](https://github.com/astriaorg/astria/pull/1802)
