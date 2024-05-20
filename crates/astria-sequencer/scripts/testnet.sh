#!/bin/sh

set -o errexit -o nounset
set -x

# Function to perform sed in-place editing in a cross-platform way
edit_in_place() {
    case "$(uname)" in
        Linux*) sed -i "$@" ;;
        Darwin*) sed -i '' "$@" ;;
        *) echo "Unsupported OS" >&2; exit 1 ;;
    esac
}

: "${OUT_DIR:=sequencer_testnet}"
: "${NUM_VALIDATORS:=4}"

# create cometbft testnet config files
cometbft testnet --v $NUM_VALIDATORS --o $OUT_DIR

# initialize cometbft config
for i in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
    APP_PORT=$((26660 + $i))
    COMETBFT_RPC_PORT=$((27660 + $i))
    COMETBFT_P2P_PORT=$((28660 + $i))

    # update sequencer app port
    edit_in_place "s/proxy_app = \"tcp:\/\/127.0.0.1:26658\"/proxy_app = \"tcp:\/\/127.0.0.1:$APP_PORT\"/g" $OUT_DIR/node$i/config/config.toml
    # RPC laddr
    edit_in_place "s/laddr = \"tcp:\/\/127.0.0.1:26657\"/laddr = \"tcp:\/\/127.0.0.1:$COMETBFT_RPC_PORT\"/g" $OUT_DIR/node$i/config/config.toml
    # p2p laddr
    edit_in_place "s/laddr = \"tcp:\/\/0.0.0.0:26656\"/laddr = \"tcp:\/\/127.0.0.1:$COMETBFT_P2P_PORT\"/g" $OUT_DIR/node$i/config/config.toml

    # update p2p persistent peer ports
    for j in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
        P2P_PORT=$((28660 + $j))
        edit_in_place "s/node$j:26656/127.0.0.1:$P2P_PORT/g" $OUT_DIR/node$i/config/config.toml
    done

    # allow peers from the same IP
    edit_in_place 's/allow_duplicate_ip = false/allow_duplicate_ip = true/' $OUT_DIR/node$i/config/config.toml
    edit_in_place 's/addr_book_strict = true/addr_book_strict = false/' $OUT_DIR/node$i/config/config.toml

    # update genesis file with app state
    ../../target/debug/astria-sequencer-utils --genesis-app-state-file=test-genesis-app-state.json --destination-genesis-file=$OUT_DIR/node$i/config//genesis.json --chain-id=astria
    sed -i'.bak' 's/timeout_commit = "1s"/timeout_commit = "2s"/g' ~/.cometbft/config/config.toml
done

# start sequencer application for each node
for i in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
    # edit default .env files for multiple validators 
    DB_PATH="/tmp/astria_db_$i"
    ASTRIA_SEQUENCER_LISTEN_ADDR=127.0.0.1:$((26660 + $i))
    ASTRIA_SEQUENCER_GRPC_ADDR=127.0.0.1:$((8080 + $i))
    ASTRIA_SEQUENCER_METRICS_HTTP_LISTENER_ADDR=127.0.0.1:$((9000 + $i))

    ASTRIA_SEQUENCER_DB_FILEPATH=$DB_PATH \
    ASTRIA_SEQUENCER_LISTEN_ADDR=$ASTRIA_SEQUENCER_LISTEN_ADDR \
    ASTRIA_SEQUENCER_GRPC_ADDR=$ASTRIA_SEQUENCER_GRPC_ADDR \
    RUST_LOG=debug RUST_BACKTRACE=1 ../../target/debug/astria-sequencer > $OUT_DIR/node$i/sequencer.log 2>&1 &

    echo $! > $OUT_DIR/node$i/sequencer.pid
done

# finally, start cometbft nodes
for i in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
    cometbft start --home $OUT_DIR/node$i > $OUT_DIR/node$i/cometbft.log 2>&1 &
done
