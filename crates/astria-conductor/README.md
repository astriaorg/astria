# Astria-Conductor

Coordinates blocks between the data availability layer and the execution layer.

## Running Conductor

### Dependencies

We use [just](https://just.systems/man/en/chapter_4.html) for convenient project
specific commands.

### Configuration
Composer is configured via environment variables. An example configuration can
be seen in `local.env.example`.

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

### Additional env variables

#### Bootnodes

You can also connect directly to a node - just add a bootnode address to the .env file

```bash
ASTRIA_CONDUCTOR_BOOTNODES="/ip4/127.0.0.1/tcp/34471/p2p/12D3KooWDCHwgGetpJuHknJqv2dNbYpe3LqgH8BKrsYHV9ALpAj8"
```

#### libp2p options

You can add a libp2p private key or port

```bash
ASTRIA_CONDUCTOR_LIBP2P_PRIVATE_KEY="{{your key}}"
ASTRIA_CONDUCTOR_LIBP2P_PORT="{{your port}}"
```

#### Celestia JWT bearer token

You can add a JWT token that's used in celestia jsonrpc calls

```bash
ASTRIA_CONDUCTOR_CELESTIA_BEARER_TOKEN="{{your token}}"
```

### Running tests

```bash
just test
```