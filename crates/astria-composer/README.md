# Astria Composer

Composer sits between a rollup execution node and the Astria Shared Sequencer to submit transactions. As currently implmented, it follows a websocket on a Geth Node for new transactions, wraps and signs the bytes into a sequencer transaction.

## Running Composer

### Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient project specific commands

### Configuration

Composer is configured via environment variables. An example configuration can be seen in `local.env.example`.

To copy a configuration to your `.env` file run, environment will default to local:

```bash
just copy-env <ENVIRONMENT>
```

### Running locally

You can use `.env` files to run easily locally. If you copied an example as above you can simply run:

```bash
just run
```