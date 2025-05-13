# Astria Charts

This directory contains Helm charts which can be used to deploy Astria
components, and run the full Astria stack.

## Dependencies

Main dependencies

* docker - <https://docs.docker.com/get-docker/>
* kubectl - <https://kubernetes.io/docs/tasks/tools/>
* helm - <https://helm.sh/docs/intro/install/>
* kind - <https://kind.sigs.k8s.io/docs/user/quick-start/#installation>
* just - <https://just.systems/man/en/chapter_4.html>

For contract deployment:

* Forge (part of Foundry) -
  <https://book.getfoundry.sh/getting-started/installation>

For funding via bridge:

* Astria Go CLI - <https://github.com/astriaorg/astria-cli-go/?tab=readme-ov-file#installation>

## Setup

In order to startup you will need to have docker running on your machine.

### Configuring Funding of Geth

By default, running this local rollup will not have any funds, but will be
configured to use the sequencer account bridge.

```bash
# create control plane cluster
just deploy cluster

# ingress controller
just deploy ingress-controller

# Deploys Sequencer + local DA
just deploy astria-local

# Deploys a geth rollup chain + faucet + blockscout + ingress
# w/ defaults running against local network, along with a bridge withdawer.
# NOTE - default values can be found in `../dev/values/rollup/dev.yaml`
just deploy rollup

# w/ custom name and id for further customization see the values file at
# `../dev/values/rollup/dev.yml`
just deploy rollup <rollup_name> <network_id>

# Send funds into the rollup chain, by default transfers 10 RIA to the rollup
# using prefunded default test sequencer accounts. 
just init rollup-bridge
# Update the amounts to init
just init rollup-bridge <rollup_name> <evm_address> <ria_amount>

# Delete default rollup
just delete rollup
# Delete custom rollup
just delete rollup <rollup_name>

# Delete the entire cluster
just clean

# Delete local persisted data (note: persisted data disabled by default)
just clean-persisted-data
```

### Faucet

The default rollup faucet is available at <http://faucet.astria.127.0.0.1.nip.io>.

If you deploy a custom faucet, it will be reachable at
`http://faucet.<rollup_name>.127.0.0.1.nip.io`.

By default, no account is funded during geth genesis.
Run `just init rollup-bridge` to fund the faucet account. This account key is
defined in `../dev/values/rollup/dev.yaml` and is identical to the key in
`./evm-rollup/files/keys/private_key.txt`.

The default sequencer faucet is available at <http://sequencer-faucet.127.0.0.1.nip.io>.

### Blockscout

The default Blockscout app is available at <http://explorer.astria.127.0.0.1.nip.io>.

If you deploy a custom Blockscout app, it will be available at
`http://explorer.<rollup_name>.127.0.0.1.nip.io`.

### Sequencer

The default sequencer RPC is available at <http://rpc.sequencer.127.0.0.1.nip.io/health>.

### EVM Rollup

The default EVM rollup has an RPC endpoint available at <http://executor.astria.127.0.0.1.nip.io>.

There is also a default WSS endpoint available at <ws://ws-executor.astria.127.0.0.1.nip.io>.

If you deploy a custom rollup, then the endpoints will be
`http://executor.<rollup_name>.127.0.0.1.nip.io` and `ws://ws-executor.<rollup_name>.127.0.0.1.nip.io`

### Connecting Metamask

* adding the default network
  * network name: `astria`
  * rpc url: `http://executor.astria.127.0.0.1.nip.io`
  * chain id: `1337`
  * currency symbol: `RIA`

* adding a custom network
  * network name: `<rollup_name>`
  * rpc url: `http://executor.<rollup_name>.127.0.0.1.nip.io`
  * chain id: `<network_id>`
  * currency symbol: `RIA`

### Using a local image

Deployment files can be updated to use a locally built docker image, for testing
of local changes. [here](https://github.com/astriaorg/astria/#docker-build).

Once you have a locally built image, update the image in the relevant deployment
to point to your local image, and upload load it into into the cluster. If you
don't already have a cluster running, first run:

```sh
# create control plane cluster
just deploy cluster
```

Then you can run the load-image command with your image name. For instance, if
we have created a local image `astria-sequencer:local`

```sh
# load image into cluster
just load-image astria-sequencer
```

To update the chart to utilize the new image, go to `./sequencer/values.yaml`
update the `images.sequencer` image repo to `astria-sequencer` and the `devTag`
to `local`. You can now deploy the chart with your local image.

## Chart Structure

### Dev vs Prod

All of our charts should run against both the latest code in monorepo AND
against the latest release. Sometimes, there are configuration changes between
releases though. To manage this in various templates you will see the following
pattern (especially in config maps and genesis files):

```yaml
{{- if not .Values.global.dev }}
// information which should be deleted after next cut release
{{- else }}
// things that are only needed for latest, should be promoted at end of release.
{{- end }}
```

## Running a Smoke Test

You can run a smoke test which ensures that full bridge functionality is working
both up and down the stack.

To deploy and run this:

```sh
# only if cluster not already created
> just deploy cluster 
# deploys all the components needed to run the test.
> just deploy smoke-test
# deploys all components needed to run the smoke test
> just run smoke-test
# Runs the smoke test will return failure if fails
> just delete smoke-test
# Clean up deployed test
```

## Running an IBC Smoke Test

You can run a smoke test which ensures that full IBC bridge functionality is
working both up and down the stack.

1. Bridges from Celestia to Astria to EVM
2. Withdraws from EVM to Astria to Celestia

```sh
> just deploy cluster
> just deploy ibc-test
> just run ibc-test
> just delete ibc-test
```

## Examining Deployments

[k9s](https://k9scli.io/) is a useful utility for inspecting deployed
containers, logs and services. Additionally, you may interact directly with the
kubernetes API using some helpful commands below.

### Helpful commands

The following commands are helpful for interacting with the cluster and its
resources. These may be useful for debugging and development, but are not
necessary for running the cluster.

```sh
# list all containers within a deployment
kubectl get -n astria-dev-cluster deployment <DEPLOYMENT_NAME> -o jsonpath='{.spec.template.spec.containers[*].name}'

# log the entire astria cluster
kubectl logs -n astria-dev-cluster -l app=astria-dev-cluster -f

# log nginx controller
kubectl logs -n ingress-nginx -f deployment/ingress-nginx-controller

# list nodes
kubectl get -n astria-dev-cluster nodes

# list pods
kubectl get --all-namespaces pods
kubectl get -n astria-dev-cluster pods

# to log a container you need to first grab the pod name from above
kubectl logs -n astria-dev-cluster -c <CONTAINER_NAME> <POD_NAME>

# delete cluster and resources
just clean

# example of deploying contract w/ forge (https://github.com/foundry-rs/foundry)
RUST_LOG=debug forge create src/Storage.sol:Storage \
  --private-key $PRIV_KEY \
  --rpc-url "http://executor.astria.127.0.0.1.nip.io"
```

### Helpful links

* <https://kubernetes.io/docs/concepts/workloads/pods/init-containers/>
* <https://kubernetes.io/docs/concepts/configuration/configmap/>
* <https://kubernetes.io/docs/reference/kubectl/cheatsheet/>
* <https://jamesdefabia.github.io/docs/user-guide/kubectl/kubectl_logs/>
