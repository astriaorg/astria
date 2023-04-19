#!/bin/sh

set -o errexit -o nounset -o pipefail

# change ports that we know metro metro will not receive messages on
# so they won't interfere with celestia-app ports:
#
# ~/.metro # netstat -lntp
# Active Internet connections (only servers)
# Proto Recv-Q Send-Q Local Address           Foreign Address         State       PID/Program name
#                     config.toml:.rpc.pprof_laddr
# tcp        0      0 127.0.0.1:6060          0.0.0.0:*               LISTEN      110/metro
#											config.toml:.rpc.laddr
# tcp        0      0 :::26657                :::*                    LISTEN      110/metro
#											p2p.laddr
# tcp        0      0 :::26656                :::*                    LISTEN      110/metro
#                     app.toml:.api.address
# tcp        0      0 :::1317                 :::*                    LISTEN      110/metro
#                     app.toml:.grpc.address
# tcp        0      0 :::9091                 :::*                    LISTEN      110/metro
#                     app.toml:.grpc-web.address
# tcp        0      0 :::9090                 :::*                    LISTEN      110/metro
dasel put -r toml '.rpc.pprof_laddr' -t string -v "127.0.0.1:60000" -f "$home_dir/config/config.toml"
dasel put -r toml '.rpc.laddr' -t string -v "tcp://0.0.0.0:60001" -f "$home_dir/config/config.toml"
dasel put -r toml '.p2p.laddr' -t string -v "tcp://0.0.0.0:60002" -f "$home_dir/config/config.toml"
dasel put -r toml '.api.address' -t string -v "tcp://0.0.0.0:1318" -f "$home_dir/config/app.toml"
dasel put -r toml '.grpc.address' -t string -v "0.0.0.0:9100" -f "$home_dir/config/app.toml"
dasel put -r toml '.grpc-web.address' -t string -v "0.0.0.0:9101" -f "$home_dir/config/app.toml"
