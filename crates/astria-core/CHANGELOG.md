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

### Changed

- Bump MSRV to 1.83.0 [#1857](https://github.com/astriaorg/astria/pull/1857).
- Move `astria_core::crypto` to `astria-core-crypto` and reexport
  `astria_core_crypto as crypto` (this change is transparent)
  [#1800](https://github.com/astriaorg/astria/pull/1800/).
- Move definitions of address domain type to `astria-core-address` and
  reexport items using the same aliases [#1802](https://github.com/astriaorg/astria/pull/1802).
- Move all Astria APIs generated from the Protobuf spec from `astria_core::generated`
  to `astria_core::generated::astria`
  [#1825](https://github.com/astriaorg/astria/pull/1825).
- Update `idna` dependency to resolve cargo audit warning [#1869](https://github.com/astriaorg/astria/pull/1869).
- Replaced all instances of `[u8; 32]` by newtype
  `astria_core::sequencerblock::v1::block::Hash` where appropriate [#1884](https://github.com/astriaorg/astria/pull/1884).

### Removed

- Remove method `TracePrefixed::last_channel` [#1768](https://github.com/astriaorg/astria/pull/1768).
- Remove method `SigningKey::try_address` [#1800](https://github.com/astriaorg/astria/pull/1800/).
- Remove inherent methods `Address::try_from_raw` and `Address::to_raw`
  [#1802](https://github.com/astriaorg/astria/pull/1802).
- Remove `AddressBuilder::with_iter` from public interface [#1802](https://github.com/astriaorg/astria/pull/1802).
