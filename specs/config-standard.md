# Configuration Standard

## Purpose

Define how configuration should be implemented across all services within Astria services.

## Background

When running a service, it is generally [best practice to store configuration in environment
variables](https://12factor.net/config), without defaults. This makes it difficult to commit
environment specific config, and ensures separation of configuration and code. It makes it easier to
integrate with KMS for secrets, and container orchestration systems are configured to enable this
easily. Not having defaults ensures configuration is done with intention on a given environment.

For local development, however, this is often inconvenient. CLI args and defaults are often easier
to manage while running on your local machine. Additionally, blockchains often have genesis files
which need to be coordinated across a diverse set of decentralized actors. Having files for this
type of configuration is often best, and the configurations may be managed via command line tools.

This standard is intended to balance the best practice, with the needs of developers for ease, and
some shared configuration.

## Astria Standard

> Note that this standard is not implemented across repo, there are tagged issues to update services
> to use the standard [here](https://github.com/astriaorg/astria/issues/240).

- Configuration in Rust managed via [Figment](https://docs.rs/figment/latest/figment/)
- Service Configuration
  - All core configuration via environment variables
    - Environment variables for each service are of the form `ASTRIA_<SERVICE>_<CONFIG_PROPERTY>`
  - All services have an example `local.env.example` in the repo
    - can be copied to `.env` to run locally
    - Examples for different environments may exist ie:
      - `devnet.env.example`
  - `.env` files are gitignored in repo
  - Tooling
    - README provides information on running locally, including copying of example env file
    - Each service will have a `justfile` to maintain ease of running
      - Loads `.env` via `set dotenv-load`
      - `just copy-env {type}` command to ease copying `.env.example` files
        - default to `local`
      - `just run` as a wrapper to `cargo run` with loaded environment
  - Shared Configuration (ie Genesis)
    - configure via passed in path to `ASTRIA_<SERVICE>_<CONFIG_FILE_TYPE>_PATH`
    - may have example files in repo, pointed to by matching `{ENV}.env.example`
      - `local.genesis.json`
- CLI Configuration
  - Configured via environment variable OR CLI arguments w/ sensible defaults as appropriate
    - defaults < environment variable < cli in terms of precedence
  - Is not a wrapper to start a service
    - The CLI should not have a `start` command which accepts args and wraps the service binary
