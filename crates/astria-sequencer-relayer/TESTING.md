# Testing in Kubernetes

`sequencer-relayer` core functionality is to retrieve blocks from
a sequencer (metro) and to relay them to a data availability layer
(celestia). To that end, its integration tests require both to
be present.

To ensure that tests are fully isolated, each test creates a unique
kubernetes namespace. In each namespace a pod comprising celestia,
an RPC node, and metro are deployed, as well as a service and ingress
rules to communicate with that pod.

To run integration tests locally, follow the steps below.

# Preparation

## Docker

The steps below assume docker is available and running. On macOS, run:
```sh
$ brew install --cask docker-desktop
```
and then start `docker-desktop` (for example, via Spotlight).

On Arch Linux (and most other distributions) follow: https://wiki.archlinux.org/title/Docker

## Just command runner

The [`justfile`](./justfile) in this repository contains the definitions
that are run below. To use it, install `just`:

```sh
# On macOS
$ brew install just

# On Arch Linux
$ sudo pacman -S just
```

## Install kind, kubectl

The integration tests were written against kind (kubernetes-in-docker`),
and CI is done by starting a kind cluster in the github workflow. This
might work with minikube and k3s, but wasn't tested.

Quick-start on kind: https://kind.sigs.k8s.io/docs/user/quick-start/

```sh
# macOS
$ brew intall kind

# Arch Linux
# using rua to install from the AUR
$ rua install kind
$ sudo pacman -S kubectl
```

## start the kind cluster

```sh
$ just create-cluster
$ kind get clusters
test-cluster
$ kubectl config current-context
kind-test-cluster
```

## deploy the nginx ingress controller and prepull required images

The ingress controller is necessary to be able to route requests to
the individual pods (and thus containers) in each of the deployments.
The images are prepulled so that they are available when deploying
the pods.

```sh
$ just deploy-ingress-controller
$ just perform-prepull
```

Deploying the ingress controller and pulling all images can take some
time, so you can wait for both steps to be completed before continuing on:

```sh
$ just wait-for-ingress-controller
$ just wait-for-prepull
```

## Running tests

The sequencer-relayer tests can then be run using:

```sh
$ cargo test --release -- --ignored
```
The `-- --ignored` is necessary because by default cargo will omit
these tests because they each take quite a bit of time.

## Cleaning up tests

Most namespaces (and the objects they contain) should be deleted
through the `Drop` impl of the `TestEnvironment` type. However,
sometimes the test completes before the a deletion request
can be sent to the Kubernetes REST API. In those cases you will see
UUID-named namespaces like this:
```sh
$ kubectl get namespaces
NAME                               STATUS   AGE
545bf8408b744afa9eeed78014ab767e   Active   53m
default                            Active   129m
ingress-nginx                      Active   129m
kube-node-lease                    Active   129m
kube-public                        Active   129m
kube-system                        Active   129m
local-path-storage                 Active   129m
```
You can delete them with
```sh
$ kubectl delete namespaces 545bf8408b744afa9eeed78014ab767e
```

## Creating an example namespaces and a sample deployment

The following steps are not necessary to run the tests themselves,
but they illustrate what cargo/the tests are doing under the hood.

```sh
# Create a namespace called "test"
$ just create-namespace

# Deploy the test environment under this namespace
$ just deploy-test-environment

# Create the ingress rule to get the latest block from the sequencer
# NOTE: This might give a 503 because metro does not immediately bind
#       the port on which it listens; wait a bit before issuing the
#       command again.
#       
$ just query-sequencer
{
  "block_id": {
    "hash": "sSpfoa73oXQddYdskOz6VGG3a6hZ+MAYTMaiY41HfnQ=",
    "part_set_header": {
      "total": 1,
      "hash": "fl7xAVccz+b6dKwdPBIZHDSl4KN9l1jLIgb7hO0eMr0="
    }
  },
  "block": {
    "header": {
      "version": {
        "block": "11",
        "app": "0"
      },
      "chain_id": "test",
      "height": "1535",
      "time": "2023-04-27T14:35:01.432976256Z",
      "last_block_id": {
        "hash": "e8+lA66E21id0jLCSZKZbqlLR2USn0L09Nx2WoeloJ8=",
        "part_set_header": {
          "total": 1,
          "hash": "GHuPppYO548tjTD/o6y7Nm5IR/YT3MyTcjL+PRTaAg8="
        }
      },
      "last_commit_hash": "it3AwnT9mT/3pnajQ3x1v1k5EkHeihs1GCDvsYW5hj0=",
      "data_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
      "validators_hash": "U5hzFMsNK9ozjoCD5LKFVux3k4GiW5tmhKPoLCWx9oM=",
      "next_validators_hash": "U5hzFMsNK9ozjoCD5LKFVux3k4GiW5tmhKPoLCWx9oM=",
      "consensus_hash": "BICRvH3cKD93v7+R1zxE2ljD34qcvIZ0Bdi389qtoi8=",
      "app_hash": "Pq8dy53jfnFnFaPleK3o++k3ajMNpusN9bZn/Wc5oxY=",
      "last_results_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
      "evidence_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
      "proposer_address": "INaK2jlx5kaWGmLEq5dHYK4y6RI="
    },
    "data": {
      "txs": [
      ],
      "blobs": [
      ],
      "square_size": "0",
      "hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
    },
    "evidence": {
      "evidence": [
      ]
    },
    "last_commit": {
      "height": "1534",
      "round": 0,
      "block_id": {
        "hash": "e8+lA66E21id0jLCSZKZbqlLR2USn0L09Nx2WoeloJ8=",
        "part_set_header": {
          "total": 1,
          "hash": "GHuPppYO548tjTD/o6y7Nm5IR/YT3MyTcjL+PRTaAg8="
        }
      },
      "signatures": [
        {
          "block_id_flag": "BLOCK_ID_FLAG_COMMIT",
          "validator_address": "INaK2jlx5kaWGmLEq5dHYK4y6RI=",
          "timestamp": "2023-04-27T14:35:01.432976256Z",
          "signature": "JBh/5ocUKFhje5E+ynersX5WYCOO4E8fTqH2LWrSCcqUgcZmkQOBFe1zmYjaoiYN7B3SiErurBmfy5S1VHuVDA=="
        }
      ]
    }
  },
  "sdk_block": {
    "header": {
      "version": {
        "block": "11",
        "app": "0"
      },
      "chain_id": "test",
      "height": "1535",
      "time": "2023-04-27T14:35:01.432976256Z",
      "last_block_id": {
        "hash": "e8+lA66E21id0jLCSZKZbqlLR2USn0L09Nx2WoeloJ8=",
        "part_set_header": {
          "total": 1,
          "hash": "GHuPppYO548tjTD/o6y7Nm5IR/YT3MyTcjL+PRTaAg8="
        }
      },
      "last_commit_hash": "it3AwnT9mT/3pnajQ3x1v1k5EkHeihs1GCDvsYW5hj0=",
      "data_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
      "validators_hash": "U5hzFMsNK9ozjoCD5LKFVux3k4GiW5tmhKPoLCWx9oM=",
      "next_validators_hash": "U5hzFMsNK9ozjoCD5LKFVux3k4GiW5tmhKPoLCWx9oM=",
      "consensus_hash": "BICRvH3cKD93v7+R1zxE2ljD34qcvIZ0Bdi389qtoi8=",
      "app_hash": "Pq8dy53jfnFnFaPleK3o++k3ajMNpusN9bZn/Wc5oxY=",
      "last_results_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
      "evidence_hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=",
      "proposer_address": "metrovaloper1yrtg4k3ew8nyd9s6vtz2h968vzhr96gjad4ws4"
    },
    "data": {
      "txs": [
      ],
      "blobs": [
      ],
      "square_size": "0",
      "hash": "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU="
    },
    "last_commit": {
      "height": "1534",
      "round": 0,
      "block_id": {
        "hash": "e8+lA66E21id0jLCSZKZbqlLR2USn0L09Nx2WoeloJ8=",
        "part_set_header": {
          "total": 1,
          "hash": "GHuPppYO548tjTD/o6y7Nm5IR/YT3MyTcjL+PRTaAg8="
        }
      },
      "signatures": [
        {
          "block_id_flag": "BLOCK_ID_FLAG_COMMIT",
          "validator_address": "INaK2jlx5kaWGmLEq5dHYK4y6RI=",
          "timestamp": "2023-04-27T14:35:01.432976256Z",
          "signature": "JBh/5ocUKFhje5E+ynersX5WYCOO4E8fTqH2LWrSCcqUgcZmkQOBFe1zmYjaoiYN7B3SiErurBmfy5S1VHuVDA=="
        }
      ]
    }
  }
}
```
