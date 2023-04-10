#!/bin/bash

# This script kills and removes docker containers that were used for integration testing.
# It also removes the network and volumes created by docker-compose.

# kill and remove containers
CONTAINER_IDS="$(docker ps -a -q \
  --filter "name=bridge0" \
  --filter "name=light0" \
  --filter "name=core0" \
  --filter "name=geth0" \
  --filter "name=metro0" \
  --filter "name=relayer0"\
  )"
echo "$CONTAINER_IDS" | xargs docker kill
echo "$CONTAINER_IDS" | xargs docker rm

# remove volumes
docker volume rm docker_keyring-volume docker_shared-volume

# remove networks
docker network rm docker_localnet
