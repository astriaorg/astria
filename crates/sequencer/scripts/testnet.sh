#!/bin/sh

set -o errexit -o nounset
set -x

: "${OUT_DIR:=sequencer_testnet}"
: "${NUM_VALIDATORS:=3}"

# create cometbft testnet config files
cometbft testnet --v $NUM_VALIDATORS --o $OUT_DIR

# initialize cometbft config
for i in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
    APP_PORT=$((26660 + $i))
    COMETBFT_RPC_PORT=$((27660 + $i))
    COMETBFT_P2P_PORT=$((28660 + $i))

    # update sequencer app port
    sed -i "s/proxy_app = \"tcp:\/\/127.0.0.1:26658\"/proxy_app = \"tcp:\/\/127.0.0.1:$APP_PORT\"/g" $OUT_DIR/node$i/config/config.toml
    # RPC laddr
    sed -i "s/laddr = \"tcp:\/\/127.0.0.1:26657\"/laddr = \"tcp:\/\/127.0.0.1:$COMETBFT_RPC_PORT\"/g" $OUT_DIR/node$i/config/config.toml
    # p2p laddr
    sed -i "s/laddr = \"tcp:\/\/0.0.0.0:26656\"/laddr = \"tcp:\/\/127.0.0.1:$COMETBFT_P2P_PORT\"/g" $OUT_DIR/node$i/config/config.toml

    # update p2p persistent peer ports
    for j in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
        P2P_PORT=$((28660 + $j))
        sed -i "s/node$j:26656/127.0.0.1:$P2P_PORT/g" $OUT_DIR/node$i/config/config.toml
    done

    # allow peers from the same IP
    sed -i 's/allow_duplicate_ip = false/allow_duplicate_ip = true/' $OUT_DIR/node$i/config/config.toml
    sed -i 's/addr_book_strict = true/addr_book_strict = false/' $OUT_DIR/node$i/config/config.toml
done

# start sequencer application for each node
for i in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
    APP_PORT=$((26660 + $i))
    ../../target/debug/astria-sequencer --genesis-file=test-genesis.json --listen-addr=127.0.0.1:$APP_PORT &> $OUT_DIR/node$i/sequencer.log &
done

# finally, start cometbft nodes
for i in $(seq 0 "$((NUM_VALIDATORS - 1))"); do
    cometbft start --home $OUT_DIR/node$i &> $OUT_DIR/node$i/cometbft.log &
done
