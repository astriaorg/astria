#!/bin/bash

set -o errexit -o nounset

geth --datadir $home_dir/.astriageth/ --http --http.addr "0.0.0.0" --http.port=8545 \
  --ws --ws.addr "0.0.0.0" --ws.port=8545 --networkid=1337 --http.corsdomain='*' --ws.origins='*' \
  --grpc --grpc.addr "0.0.0.0" --grpc.port 50051 \
  --metro.addr "0.0.0.0" --metro.port 9100
