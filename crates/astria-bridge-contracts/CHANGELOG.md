<!-- markdownlint-disable no-duplicate-heading -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

- Read the provided contract's `decimals` function, falling back to a hardcoded
  value of 18 if the call fails.
  [#1762](https://github.com/astriaorg/astria/pull/1762)

### Added

- Initial release.

### Fixed

- Fixed ICS20 withdrawal source when using channel with more than one
  port/channel combo. [#1768](https://github.com/astriaorg/astria/pull/1768)
