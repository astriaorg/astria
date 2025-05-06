import 'defaults.just'

# Deleting clusters and persisted data. 'just --list clean' for more...
mod clean
# Deleting Helm/K8s deployments. 'just --list delete' for more...
mod delete
# Deployments using Helm and K8s. 'just --list helm' for more...
mod deploy
# Interacting with EVM. 'just --list evm' for more...
mod evm
# Initialization of various components. 'just --list init' for more...
mod init
# Running tests and other operations. 'just --list run' for more...
mod run
# Awaiting deployments. `$just --list wait-for` for more...
mod wait-for


##############################################
## Deploying and Running using Helm and K8s ##
##############################################


# Load Docker Image
####################
[doc("
Loads a docker image into a running K8s kind cluster.
Note: the image must be built and cluster must be running before this recipe can
be called.
Usage:
  just load-image [IMAGE_NAME] <TAG> <REPO> <NAMESPACE>
    defaults:
      TAG: ''
      REPO: 'ghcr.io/astriaorg'
      NAMESPACE: 'astria-dev-cluster'
Full example:
  just deploy cluster
  just docker-build astria-sequencer
  just load-image sequencer
")]
load-image image_name tag=defaultDockerTag repo=default_repo_name namespace=defaultNamespace:
  kind load docker-image {{repo}}/{{image_name}}:{{tag}} --name {{namespace}}
