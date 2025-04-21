import 'defaults.just'


######################
## Cleaning Recipes ##
######################


# Clean Cluster
################
[doc("
Deletes Kubernetes cluster.
Usage:
  just clean
  just clean cluster <NAMESPACE> (default: 'astria-dev-cluster')
")]
cluster namespace=defaultNamespace:
  @kind delete cluster --name {{namespace}}


# Clean Helm
#############
[doc("
Cleans the cluster without deleting it, and deletes non-core persisted data.
Note: there is no persisted data by default.
")]
helm:
  helm ls -a --all-namespaces -l core!=true | awk 'NR > 1 { print  "-n "$2, $1}' | xargs -L1 helm uninstall
  just clean persisted-data


# Clean Persisted Data
#######################
[doc("
Deletes non-core persisted data from K8s cluster deployed using 'deploy::cluster'.
Note: there is no persisted data by default.
")]
persisted-data:
  rm -rf /tmp/astria/celestia-data/*
  rm -rf /tmp/astria/sequencer-data/*
  rm -rf /tmp/astria/sequencer-relayer-data/*
  rm -rf /tmp/astria/rollup-data/*
