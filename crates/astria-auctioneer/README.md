# Astria Auctioneer

TODO: Add a description of the binary.

## Running The Auctioneer

### Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient project
specific commands.

### Configuration

The Auctioneer is configured via environment variables. An example configuration
can be seen in `local.env.example`.

To copy a configuration to your `.env` file run:

```sh

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
