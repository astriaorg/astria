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
  @kubectl wait -n {{defaultNamespace}} deployment evm-bridge-withdrawer-local --for=condition=Available=True --timeout=600s > /dev/null


# Waits for local Celestia instance
###################################
[doc("
Waits for the local Celestia instance to be ready.
")]
celestia-local:
  @echo "Waiting for local Celestia instance to be ready..."
  @kubectl rollout status --watch statefulset/celestia-local -n {{defaultNamespace}} --timeout=600s > /dev/null


# Waits for Hermes
###################
[doc("
Waits for Hermes to be ready.
")]
hermes-local:
  @echo "Waiting for Hermes to be ready..."
  @kubectl wait -n {{defaultNamespace}} deployment hermes-local-chart --for=condition=Available=True --timeout=600s


# Waits for ingress controller
###############################
[doc("
Waits for the ingress controller to be ready.
")]
ingress-controller:
  @echo "Waiting for ingress controller to be ready..."
  @until kubectl wait -n ingress-nginx --for=condition=ready pod --selector=app.kubernetes.io/component=controller --timeout=600s; do \
    sleep 1; \
  done


withdrawerEnabled := "true"
# Wait for Rollup
##################
[doc("
Waits for the rollup to be ready.
Note: This recipe can also be used to await readiness of `flame-dev-rollup`.
Usage:
  just wait-for-rollup <ROLLUP_NAME> <BRIDGE_WITHDRAWER_ENABLED>
  (defaults: 'astria', 'true')
")]
rollup rollupName=defaultRollupName bridgeWithdrawerEnabled=withdrawerEnabled:
  @echo "Waiting for rollup '{{rollupName}}' to be ready..."
  @kubectl rollout status --watch statefulset/{{rollupName}}-geth -n {{defaultNamespace}} --timeout=600s > /dev/null
  {{ if bridgeWithdrawerEnabled == "true" { 'just wait-for bridge-withdrawer' } else { '' } }}


# Waits for Sequencer
#####################
[doc("
Waits for the sequencer to be ready.
")]
sequencer:
  @echo "Waiting for sequencer to be ready..."
  @kubectl rollout status --watch statefulset/sequencer -n {{defaultNamespace}} --timeout=600s > /dev/null


# Waits for Bridge Signer
##########################
[doc("
Waits for the Bridge Signer to be ready.
Usage:
  just wait-for bridge-signer <SIGNER_NAME>
  (defaults: 'signer1')
")]
bridge-signer name=signerName:
  @kubectl wait -n astria-signer-{{name}} deployment bridge-signer-{{name}} --for=condition=Available=True --timeout=600s > /dev/null


# Waits for Rollup Transaction Indexing
########################################
[doc("
Waits for rollup to finish indexing transactions.
")]
rollup-transaction-index:
  #!/usr/bin/env bash
  echo "Waiting for rollup to finish indexing transactions..."
  while true; do
    SYNC_STATUS=$(just evm get-sync-status 2>/dev/null || echo "error")
    if [[ $SYNC_STATUS == "false" ]]; then
      echo "Rollup has finished indexing transactions."
      break
    else
      echo "Rollup still indexing..."
      sleep 3
    fi
  done
