# Astria Account Monitor

The Account Monitor service continuously tracks account
states on the Astria Shared Sequencer. It monitors:

- Regular account balances and nonces
- Bridge account transaction heights

The service runs a simple loop that periodically queries account information
and updates metrics. This allows for real-time monitoring of account activity
and health checks on the network.

## Running Account Monitor

### Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient
project-specific commands.

### Configuration

Account Monitor is configured via environment variables.
An example configuration can be seen in `local.env.example`.

To copy a configuration to your `.env` file run:

```sh
# By default will copy `local.env.example`
just copy-env
```

### Running locally

After creating a `.env` file either manually or by copying as above, `just` will
load it and run locally:

```bash
just run
```
