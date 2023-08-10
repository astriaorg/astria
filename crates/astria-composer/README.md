# Astria Composer

Composer sits between a rollup execution node and the Astria Shared Sequencer to submit
transactions. As currently implemented, it follows a websocket on a Geth Node for new transactions,
wraps and signs the bytes into a sequencer transaction. Composer currently supports gathering
transactions from a single rollup. In the future it may support multiple rollups.

## Running Composer

### Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient project specific commands.

### Configuration

Composer is configured via environment variables. An example configuration can be seen in
`local.env.example`.

To copy a configuration to your `.env` file run:

```bash

# Can specify an environment
just copy-env <ENVIRONMENT>

# By default will copy `local.env.example`
just copy-env
```

### Running locally

After creating a `.env` file either manually or by copying as above, `just` will load it and run
locally:

```bash
just run
```
