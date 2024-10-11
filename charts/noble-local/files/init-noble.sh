#!/bin/sh

set -o errexit -o nounset

ls -la "$home_dir"

KEYRING="--keyring-backend=$keyring_backend"

# TODO - move to config?
DENOM="uusdc"
BASEDENOM="u$DENOM"
# FIXME - how necessary is this stuff for testing ibc?
TF1_MINTING_DENOM="steeze"
TF1_MINTING_BASEDENOM="u$TF1_MINTING_DENOM"
TF2_MINTING_DENOM="rupees"
TF2_MINTING_BASEDENOM="u$TF2_MINTING_DENOM"

# init chain
nobled --home "$home_dir" \
  init "$chainid" \
  --chain-id "$chainid"

nobled --home "$home_dir" config chain-id "$chainid"

# add keys and genesis accounts
# validator
echo "$validator_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add "$validator_key_name" --recover
# token factory 1 owner
echo "$tf1_owner_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add tf1_owner --recover
# token factory 1 owner
echo "$tf2_owner_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add tf2_owner --recover
# ibc account
echo "$ibc_account_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add "$ibc_account_key_name" --recover
# account used for development
echo "$dev_account_mnemonic" | nobled --home "$home_dir" "$KEYRING" \
  keys add "$dev_account_key_name" --recover

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
IBC_ACCOUNT=$(nobled --home "$home_dir" keys "$KEYRING" show "$ibc_account_key_name" -a)

# configuration changes
echo "updating config.toml with sed"
# FIXME - use dasel? all the other cosmos apps use sed like this though so it's sort of conventional
sed -i 's#"tcp://127.0.0.1:26657"#"tcp://0.0.0.0:'"$noble_rpc_port"'"#g' "$home_dir/config/config.toml"
sed -i 's#"tcp://0.0.0.0:26656"#"tcp://0.0.0.0:'"$noble_p2p_port"'"#g' "$home_dir/config/config.toml"
sed -i 's#"localhost:6060"#"localhost:'"$noble_p2p_port"'"#g' "$home_dir/config/config.toml"
sed -i 's/timeout_commit = "500ms"/timeout_commit = "5s"/g' "$home_dir/config/config.toml"
sed -i 's/timeout_propose = "3s"/timeout_propose = "1s"/g' "$home_dir/config/config.toml"
sed -i 's/index_all_keys = false/index_all_keys = true/g' "$home_dir/config/config.toml"
sed -i 's/"authority": ""/"authority": "'"$TF1_OWNER"'"/g' "$home_dir/config/genesis.json"
sed -i 's/chain-id = ""/chain-id = "noble-local-0"/g' "$home_dir/config/client.toml"

# FIXME - remove after dev
cat "$home_dir/config/genesis.json"

echo "updating genesis.json with jq"
# cd to home_dir because we have permission here to touch
cd $home_dir
TMPGEN=tempGen.json
# update bond_denom
jq --arg DENOM "$DENOM" '.app_state.staking.params.bond_denom = $DENOM' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
# update denom_metadata
jq --arg TF1_MINTING_DENOM "$TF1_MINTING_DENOM" \
   --arg TF1_MINTING_BASEDENOM "$TF1_MINTING_BASEDENOM" \
   --arg TF2_MINTING_DENOM "$TF2_MINTING_DENOM" \
   --arg TF2_MINTING_BASEDENOM "$TF2_MINTING_BASEDENOM" \
   --arg DENOM "$DENOM" \
   --arg BASEDENOM "$BASEDENOM" \
   '.app_state.bank.denom_metadata = [
     {
       "display": $DENOM,
       "base": $BASEDENOM,
       "name": $DENOM,
       "symbol": $DENOM,
       "denom_units": [
         {"denom": $DENOM, "aliases": ["micro\($DENOM)"], "exponent": "0"},
         {"denom": "m\($DENOM)", "aliases": ["mili\($DENOM)"], "exponent": "3"},
         {"denom": $BASEDENOM, "aliases": null, "exponent": "6"}
       ]
     },
     {
       "display": $TF1_MINTING_DENOM,
       "base": $TF1_MINTING_BASEDENOM,
       "name": $TF1_MINTING_DENOM,
       "symbol": $TF1_MINTING_DENOM,
       "denom_units": [
         {"denom": $TF1_MINTING_DENOM, "aliases": ["micro\($TF1_MINTING_DENOM)"], "exponent": "0"},
         {"denom": "m\($TF1_MINTING_DENOM)", "aliases": ["mili\($TF1_MINTING_DENOM)"], "exponent": "3"},
         {"denom": $TF1_MINTING_BASEDENOM, "aliases": null, "exponent": "6"}
       ]
     },
     {
       "display": $TF2_MINTING_DENOM,
       "base": $TF2_MINTING_BASEDENOM,
       "name": $TF2_MINTING_DENOM,
       "symbol": $TF2_MINTING_DENOM,
       "denom_units": [
         {"denom": $TF2_MINTING_DENOM, "aliases": ["micro\($TF2_MINTING_DENOM)"], "exponent": "0"},
         {"denom": "m\($TF2_MINTING_DENOM)", "aliases": ["mili\($TF2_MINTING_DENOM)"], "exponent": "3"},
         {"denom": $TF2_MINTING_BASEDENOM, "aliases": null, "exponent": "6"}
       ]
     }
   ]' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"

# update all the "authority" values
jq --arg TF1_OWNER "$TF1_OWNER" '.app_state.mint.minter.authority = $TF1_OWNER' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq --arg TF1_OWNER "$TF1_OWNER" '.app_state.params.params.authority = $TF1_OWNER' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq --arg TF1_OWNER "$TF1_OWNER" '.app_state.upgrade.params.authority = $TF1_OWNER' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq --arg IBC_ACCOUNT "$IBC_ACCOUNT" '.app_state."ibc-authority".params.authority = $IBC_ACCOUNT' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"

jq --arg TF1_OWNER "$TF1_OWNER" '.app_state.tokenfactory.owner.address = $TF1_OWNER' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq --arg TF1_MINTING_BASEDENOM "$TF1_MINTING_BASEDENOM" '.app_state.tokenfactory.mintingDenom.denom = $TF1_MINTING_BASEDENOM' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq '.app_state.tokenfactory.paused.paused = false' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"

jq --arg TF2_OWNER "$TF2_OWNER" '.app_state."fiat-tokenfactory".owner.address = $TF2_OWNER' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq --arg TF1_MINTING_BASEDENOM "$TF1_MINTING_BASEDENOM" '.app_state."fiat-tokenfactory".mintingDenom.denom = $TF1_MINTING_BASEDENOM' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
jq '.app_state."fiat-tokenfactory".paused.paused = false' "$home_dir/config/genesis.json" > "$TMPGEN" && mv "$TMPGEN" "$home_dir/config/genesis.json"
