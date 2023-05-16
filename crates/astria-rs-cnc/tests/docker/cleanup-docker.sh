#!/bin/bash

# This script kills and removes docker containers that were used for integration testing.
# It also removes the network created by docker-compose.

docker kill $(docker ps -a -q --filter "name=bridge0" --filter "name=light0" --filter "name=core0")
docker rm $(docker ps -a -q --filter "name=bridge0" --filter "name=light0" --filter "name=core0")
docker network rm docker_localnet
