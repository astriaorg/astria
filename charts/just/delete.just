import 'defaults.just'
import 'clean.just'


_default_delete:
  @just --list delete


######################
## Deletion Recipes ##
######################


# Delete All
#############
[doc("
Deletes all resources via deletion of the Kubernetes cluster and any associated
persisted data.
")]
all:
  @just clean cluster
  @just clean persisted-data


# Delete Astria Local
######################
[doc("
Deletes local Sequencer and Celestia deployments.
Usage:
  just delete astria-local <NAMESPACE> (default: 'astria-dev-cluster')
")]
astria-local namespace=defaultNamespace: delete-celestia-local sequencer


# Delete Celestia Local
######################
[doc("
Deletes local Celestia deployments.
Usage:
  just delete celestia-local <NAMESPACE> (default: 'astria-dev-cluster')
")]
celestia-local namespace=defaultNamespace: delete-celestia-local


# Delete Graph Node
####################
[doc("
Deletes local graph node.
Usage:
  just delete graph-node <NAMESPACE> (default: 'astria-dev-cluster')
")]
graph-node namespace=defaultNamespace: (delete-chart "graph-node" namespace)


# Delete Graph Node PVC
########################
[doc("
Deletes graph node persisted volumes.
Usage:
  just delete graph-node-pvc <NAMESPACE> (default: 'astria-dev-cluster')
")]
graph-node-pvc namespace=defaultNamespace:
  @kubectl delete pvc -n namespace \
    -l 'app.kubernetes.io/instance=graph-node-chart' \
    -o name | xargs -r kubectl delete -n ${namespace} && \
  @kubectl delete pvc -n namespace \
    -l 'app.kubernetes.io/managed-by=Helm' \
    --field-selector 'metadata.name in (ipfs-pvc,postgres-pvc)'


# Delete Secrets Store
#######################
[doc("
Deletes local secrets store.
")]
secrets-store: (delete-chart "csi-secrets-store" "kube-system")


# Delete Astrotrek
###################
[doc("
Deletes local Astrotrek deployment.
")]
astrotrek: (delete-chart "astrotrek")


# Delete Hermes
################
[doc("
Deletes local hermes deployment.
")]
hermes-local: (delete-chart "hermes-local")


# Delete Sequencer
###################
[doc("
Deletes local Sequencer deployment.
Usage:
  just delete sequencer <NAME> (default: 'single')
")]
sequencer name=validatorName:
  @just delete::delete-chart {{name}}-sequencer astria-validator-{{name}}


# Delete Sequencers
####################
[doc("
Deletes all Sequencers deployed by 'deploy::sequencers' (node0, node1, and node2
in 'dev/values/validators').
")]
sequencers: (sequencer "node0") (sequencer "node1") (sequencer "node2") (sequencer "node3") (sequencer "node4")

# Delete Rollup
################
[doc("
Deletes local EVM rollup stack deployment.
Usage:
  just delete rollup <ROLLUP_NAME> (default: 'astria')
")]
rollup rollupName=defaultRollupName:
  @just delete::delete-chart {{rollupName}}-chain


# Delete Flame Rollup
######################
[doc("
Deletes local Flame rollup deployment.
Usage:
  just delete flame-dev-rollup <ROLLUP_NAME> (default: 'astria')
")]
flame-dev-rollup rollupName=defaultRollupName: (rollup rollupName)


# Delete Auctioneer
####################
[doc("
Deletes auctioneer deployment.
")]
auctioneer: (delete-chart "auctioneer")


# Delete Bridge Withdrawer
###########################
[doc("
Deletes local EVM bridge-withdrawer deployment.
")]
bridge-withdrawer: (delete-chart "evm-bridge-withdrawer")

# Delete Account Monitor
[doc("
Deletes local account-monitor deployment.
")]
account-monitor: (delete-chart "account-monitor")

# Delete Hyperlane Agents
##########################
[doc("
Deletes Hyperlane agents on a locally running rollup.
Usage:
  just delete hyperlane-agents <ROLLUP_NAME> (default: 'astria')
")]
hyperlane-agents rollupName=defaultRollupName:
  @just delete::delete-chart {{rollupName}}-hyperlane-agents


# Delete Smoke Test
####################
[doc("
Deletes associated components for running smoke tests (Celestia, Sequencer, and
EVM rollup).
")]
smoke-test: delete-celestia-local
  @just delete sequencer
  @just delete rollup


# Delete IBC Test
##################
[doc("
Deletes all components for running the IBC test.
")]
ibc-test: delete-celestia-local
  @just delete sequencer
  @just delete hermes-local
  @just delete rollup


#####################
## Private recipes ##
#####################

[private]
delete-chart chart namespace=defaultNamespace:
  @helm uninstall {{chart}}-chart --namespace {{namespace}}

[private]
delete-celestia-local namespace=defaultNamespace: (delete-chart "celestia-local" namespace)
