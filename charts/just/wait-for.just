import 'defaults.just'


_default_wait_for:
  @just --list wait-for


#################################
## Wait for Deployment Recipes ##
#################################


# Waits for EVM Bridge Withdrawer
##################################
[doc("
Waits for the EVM Bridge Withdrawer to be ready.
")]
bridge-withdrawer:
  @echo "Waiting for EVM Bridge Withdrawer to be ready..."
  @kubectl wait -n astria-dev-cluster deployment evm-bridge-withdrawer-local --for=condition=Available=True --timeout=600s > /dev/null


# Waits for local Celestia instance
###################################
[doc("
Waits for the local Celestia instance to be ready.
")]
celestia-local:
  @echo "Waiting for local Celestia instance to be ready..."
  @kubectl rollout status --watch statefulset/celestia-local -n astria-dev-cluster --timeout=600s > /dev/null


# Waits for Hermes
###################
[doc("
Waits for Hermes to be ready.
")]
hermes-local:
  @echo "Waiting for Hermes to be ready..."
  @kubectl wait -n astria-dev-cluster deployment hermes-local-chart --for=condition=Available=True --timeout=480s


# Waits for ingress controller
###############################
[doc("
Waits for the ingress controller to be ready.
")]
ingress-controller:
  @echo "Waiting for ingress controller to be ready..."
  @while ! kubectl wait --namespace ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s; do \
    sleep 1; \
  done


withdrawerEnabled := "true"
# Wait for Rollup
##################
[doc("
Waits for the rollup to be ready.
Note: This recipe can also be used to await readiness of `flame-dev-rollup`.
Usage:
  $just wait-for-rollup <ROLLUP_NAME> <BRIDGE_WITHDRAWER_ENABLED>
    defaults:
      ROLLUP_NAME: 'defaultRollupName'
      BRIDGE_WITHDRAWER_ENABLED: 'true'
")]
rollup rollupName=defaultRollupName bridgeWithdrawerEnabled=withdrawerEnabled:
  @echo "Waiting for rollup `{{rollupName}}`` to be ready..."
  @kubectl rollout status --watch statefulset/{{rollupName}}-geth -n astria-dev-cluster --timeout=600s > /dev/null
  {{ if bridgeWithdrawerEnabled == "true" { 'just wait-for bridge-withdrawer' } else { '' } }}


# Waits for Sequencer
#####################
[doc("
Waits for the sequencer to be ready.
")]
sequencer:
  @echo "Waiting for sequencer to be ready..."
  @kubectl rollout status --watch statefulset/sequencer -n astria-dev-cluster --timeout=600s > /dev/null
