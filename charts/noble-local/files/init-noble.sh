#!/bin/sh

set -o errexit -o nounset

KEYRING="--keyring-backend=$keyring_backend"

# TODO - move to config?
DENOM="uusdc"
BASEDENOM="u$DENOM"

echo "Initializing noble node..."
nobled --home "$home_dir" init "$chainid" --chain-id "$chainid"
nobled keys add validator --home $home_dir "$KEYRING"
# add keys and genesis accounts
# validator
# ibc account
echo "$ibc_account_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add "$ibc_account_key_name" --recover
# account used for development
echo "$dev_account_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add "$dev_account_key_name" --recover
nobled genesis --home "$home_dir" "$KEYRING" \
  add-genesis-account "$(nobled --home "$home_dir" keys "$KEYRING" show "$ibc_account_key_name" -a)" "$coins"
nobled genesis --home "$home_dir" "$KEYRING" \
  add-genesis-account "$(nobled --home "$home_dir" keys "$KEYRING" show "$dev_account_key_name" -a)" "$coins"

nobled genesis add-genesis-account validator 1000000ustake --home $home_dir "$KEYRING"
AUTHORITY=$(nobled keys add authority --home $home_dir --keyring-backend test --output json | jq .address)
echo "Authority address: $AUTHORITY"
nobled genesis add-genesis-account authority 4000000ustake --home $home_dir "$KEYRING"

TEMP=$home_dir/genesis.json
touch $TEMP && jq '.app_state.authority.owner = '$AUTHORITY'' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
touch $TEMP && jq '.app_state.bank.denom_metadata += [{ "description": "Circle USD Coin", "denom_units": [{ "denom": "uusdc", "exponent": 0, "aliases": ["microusdc"] }, { "denom": "usdc", "exponent": 6 }], "base": "uusdc", "display": "usdc", "name": "Circle USD Coin", "symbol": "USDC" }]' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
# touch $TEMP && jq '.app_state.bank.denom_metadata += [{ "description": "Ondo US Dollar Yield", "denom_units": [{ "denom": "ausdy", "exponent": 0, "aliases": ["attousdy"] }, { "denom": "usdy", "exponent": 18 }], "base": "ausdy", "display": "usdy", "name": "Ondo US Dollar Yield", "symbol": "USDY" }]' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
# touch $TEMP && jq '.app_state.bank.denom_metadata += [{ "description": "Hashnote US Yield Coin", "denom_units": [{ "denom": "uusyc", "exponent": 0, "aliases": ["microusyc"] }, { "denom": "usyc", "exponent": 6 }], "base": "uusyc", "display": "usyc", "name": "Hashnote US Yield Coin", "symbol": "USYC" }]' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
# touch $TEMP && jq '.app_state.bank.denom_metadata += [{ "description": "Monerium EUR emoney", "denom_units": [{ "denom": "ueure", "exponent": 0, "aliases": ["microeure"] }, { "denom": "eure", "exponent": 6 }], "base": "ueure", "display": "eure", "name": "Monerium EUR emoney", "symbol": "EURe" }]' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
touch $TEMP && jq '.app_state."fiat-tokenfactory".mintingDenom = { "denom": "uusdc" }' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
touch $TEMP && jq '.app_state.staking.params.bond_denom = "ustake"' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json
# set pause flag to false
touch $TEMP && jq '.app_state."fiat-tokenfactory".paused = { "paused": false }' $home_dir/config/genesis.json > $TEMP && mv $TEMP $home_dir/config/genesis.json

sed -i 's/chain-id = ""/chain-id = "noble-local-0"/g' "$home_dir/config/client.toml"
sed -i 's#"tcp://127.0.0.1:26657"#"tcp://0.0.0.0:'"$noble_rpc_port"'"#g' "$home_dir/config/config.toml"
sed -i 's#"tcp://0.0.0.0:26656"#"tcp://0.0.0.0:'"$noble_p2p_port"'"#g' "$home_dir/config/config.toml"
sed -i 's#"localhost:6060"#"localhost:'"$noble_p2p_port"'"#g' "$home_dir/config/config.toml"
sed -i 's/timeout_commit = "500ms"/timeout_commit = "1s"/g' "$home_dir/config/config.toml"
sed -i 's/timeout_propose = "3s"/timeout_propose = "1s"/g' "$home_dir/config/config.toml"

nobled genesis gentx validator 1000000ustake --chain-id "$chainid" --home $home_dir "$KEYRING" &> /dev/null
nobled genesis collect-gentxs --home $home_dir &> /dev/null

