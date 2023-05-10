#!/bin/bash

set -o errexit -o nounset

geth --datadir $home_dir/.astriageth/ --http --http.addr "0.0.0.0" --http.port=$executor_host_http_port \
  --ws --ws.addr "0.0.0.0" --ws.port=$executor_host_http_port --networkid=1337 --http.corsdomain='*' --ws.origins='*' \
  --grpc --grpc.addr "0.0.0.0" --grpc.port $executor_host_grpc_port \
  --metro.addr "0.0.0.0" --metro.port $sequencer_host_grpc_port
