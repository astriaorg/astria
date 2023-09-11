# Astria Sequencer-Relayer

Sequencer-Relayer reads new blocks from [Astria Sequencer](../astria-sequencer)
(run as a proxy app of cometBFT), submits them to celestia (which is used as a
data availability layer), and gossips them over P2P, where they are usually read
by [Astria Conductor](../astria-conductor).

## Running Sequencer-Relayer

### Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient project
specific commands.

### Configuration

Sequencer-Relayer is configured via environment variables. An example
configuration can be seen in `local.env.example`.

To copy a configuration to your `.env` file run:

```bash
# Can specify an environment
just copy-env <ENVIRONMENT>

# By default will copy `local.env.example`
just copy-env
```

### Running locally

After creating a `.env` file either manually or by copying as above, `just` will
load it and run locally:

```bash
just run
```
