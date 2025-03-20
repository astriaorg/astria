import 'defaults.just'


######################
## Cleaning Recipes ##
######################


# Clean Cluster
################
[doc("
Deletes Kubernetes cluster.
Usage:
  $just clean
  $just clean cluster <NAMESPACE> (default: 'astria-dev-cluster')
")]
cluster namespace=defaultNamespace:
  @kind delete cluster --name {{namespace}}


# Clean Persisted Data
#######################
[doc("
Deletes persisted data from K8s cluster deployed using 'deploy::cluster'.
Note: there is no persisted data by default.
")]
persisted-data:
  rm -r /tmp/astria
