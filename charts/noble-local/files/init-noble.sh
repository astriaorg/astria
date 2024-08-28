#!/bin/sh

set -o errexit -o nounset

ls -la "$home_dir"
cat "$home_dir/config/genesis.json" | jq ".genesis_time,.chain_id"

KEYRING="--keyring-backend=$keyring_backend"

# TODO - move to config?
TF1_MINTING_DENOM='steeze'
TF1_MINTING_BASEDENOM="u$TF1_MINTING_DENOM"
TF2_MINTING_DENOM='rupees'
TF2_MINTING_BASEDENOM="u$TF2_MINTING_DENOM"

# init chain
# NOTE - had to use --overwrite here because `/home/noble/config/genesis.json` already exists
#  for some reason??? and this image doesn't have rm??? wtf
nobled --home "$home_dir" \
  init "$chainid" --chain-id "$chainid" --overwrite

# add keys and genesis accounts
# validator
echo "$validator_mnemonic" | nobled \
  keys add "$validator_key_name" --recover --home "$home_dir" "$KEYRING"
# token factory 1 owner
echo "$tf1_owner_mnemonic" | nobled \
  keys add tf1_owner --recover --home "$home_dir" "$KEYRING"
# token factory 1 owner
echo "$tf2_owner_mnemonic" | nobled \
  keys add tf2_owner --recover --home "$home_dir" "$KEYRING"
# ibc account
echo "$ibc_account_mnemonic" | nobled \
  keys add "$ibc_account_key_name" --recover --home "$home_dir" "$KEYRING"
# account used for development
echo "$dev_account_mnemonic" | nobled \
  keys add "$dev_account_key_name" --recover --home "$home_dir" "$KEYRING"

nobled --home "$home_dir" "$KEYRING" \
  add-genesis-account "$(nobled --home "$home_dir" keys "$KEYRING" show tf1_owner -a)" "$coins"
nobled --home "$home_dir" "$KEYRING" \
  add-genesis-account "$(nobled --home "$home_dir" keys "$KEYRING" show "$validator_key_name" -a)" "$coins"
nobled --home "$home_dir" "$KEYRING" \
  add-genesis-account "$(nobled --home "$home_dir" keys "$KEYRING" show "$ibc_account_key_name" -a)" "$coins"
nobled --home "$home_dir" "$KEYRING" \
  add-genesis-account "$(nobled --home "$home_dir" keys "$KEYRING" show "$dev_account_key_name" -a)" "$coins"

nobled --home "$home_dir" "$KEYRING" \
  gentx "$validator_key_name" "$validator_stake" --chain-id "$chainid"

nobled --home "$home_dir" \
  collect-gentxs

TF1_OWNER=$(nobled --home "$home_dir" keys "$KEYRING" show tf1_owner -a)
TF2_OWNER=$(nobled --home "$home_dir" keys "$KEYRING" show tf2_owner -a)

# FIXME - DEBUGGING
cat "$home_dir/config/genesis.json" | jq ".app_state.upgrade"
TF1_OWNER=$(nobled --home "$home_dir" keys "$KEYRING" show tf1_owner -a)
TF2_OWNER=$(nobled --home "$home_dir" keys "$KEYRING" show tf2_owner -a)
echo "***********************"
echo "TF1_OWNER: $TF1_OWNER"
echo "TF2_OWNER: $TF2_OWNER"

# configuration changes
sed -i 's#"tcp://127.0.0.1:26657"#"tcp://0.0.0.0:'"$noble_rpc_port"'"#g' "$home_dir/config/config.toml"
sed -i 's#"tcp://0.0.0.0:26656"#"tcp://0.0.0.0:'"$noble_p2p_port"'"#g' "$home_dir/config/config.toml"
sed -i 's#"localhost:6060"#"localhost:'"$noble_p2p_port"'"#g' "$home_dir/config/config.toml"
sed -i 's/timeout_commit = "5s"/timeout_commit = "1s"/g' "$home_dir/config/config.toml"
sed -i 's/timeout_propose = "3s"/timeout_propose = "1s"/g' "$home_dir/config/config.toml"
sed -i 's/index_all_keys = false/index_all_keys = true/g' "$home_dir/config/config.toml"
sed -i 's/"bond_denom": "stake"/"bond_denom": "'"$DENOM"'"/g' "$home_dir/config/genesis.json"
sed -i 's/"denom_metadata": \[]/"denom_metadata": [ { "display": "'$TF1_MINTING_DENOM'", "base": "'$TF1_MINTING_BASEDENOM'", "name": "'$TF1_MINTING_DENOM'", "symbol": "'$TF1_MINTING_DENOM'", "denom_units": [ { "denom": "'$TF1_MINTING_DENOM'", "aliases": [ "micro'$TF1_MINTING_DENOM'" ], "exponent": "0" }, { "denom": "m'$TF1_MINTING_DENOM'", "aliases": [ "mili'$TF1_MINTING_DENOM'" ], "exponent": "3" }, { "denom": "'$TF1_MINTING_BASEDENOM'", "aliases": null, "exponent": "6" } ] }, { "display": "'$TF2_MINTING_DENOM'", "base": "'$TF2_MINTING_BASEDENOM'", "name": "'$TF2_MINTING_DENOM'", "symbol": "'$TF2_MINTING_DENOM'", "denom_units": [ { "denom": "'$TF2_MINTING_DENOM'", "aliases": [ "micro'$TF2_MINTING_DENOM'" ], "exponent": "0" }, { "denom": "m'$TF2_MINTING_DENOM'", "aliases": [ "mili'$TF2_MINTING_DENOM'" ], "exponent": "3" }, { "denom": "'$TF2_MINTING_BASEDENOM'", "aliases": null, "exponent": "6" } ] } ]/g' "$home_dir/config/genesis.json"
sed -i 's/"authority": ""/"authority": '"$TF1_OWNER"'/g' "$home_dir/config/genesis.json"

TMPGEN=tempGen.json
touch $TMPGEN && jq '.app_state.tokenfactory.owner.address = '"$TF1_OWNER"'' "$home_dir/config/genesis.json" > $TMPGEN && mv $TMPGEN "$home_dir/config/genesis.json"
touch $TMPGEN && jq '.app_state.tokenfactory.mintingDenom.denom = "'$TF1_MINTING_BASEDENOM'"' "$home_dir/config/genesis.json" > $TMPGEN && mv $TMPGEN "$home_dir/config/genesis.json"
touch $TMPGEN && jq '.app_state.tokenfactory.paused.paused = false' "$home_dir/config/genesis.json" > $TMPGEN && mv $TMPGEN "$home_dir/config/genesis.json"

touch $TMPGEN && jq '.app_state."fiat-tokenfactory".owner.address = '"$TF2_OWNER"'' "$home_dir/config/genesis.json" > $TMPGEN && mv $TMPGEN "$home_dir/config/genesis.json"
touch $TMPGEN && jq '.app_state."fiat-tokenfactory".mintingDenom.denom = "'$TF1_MINTING_BASEDENOM'"' "$home_dir/config/genesis.json" > $TMPGEN && mv $TMPGEN "$home_dir/config/genesis.json"
touch $TMPGEN && jq '.app_state."fiat-tokenfactory".paused.paused = false' "$home_dir/config/genesis.json" > $TMPGEN && mv $TMPGEN "$home_dir/config/genesis.json"
