# ArgoCD Guide

ArgoCD apps and appsets can be applied and edited with the ArgoCD CLI or with kubectl.

## Kubectl

To apply an app or appset with kubectl, use the following command:

    kubectl -n argocd apply -f <APP_OR_APPSET_YAML>
Note:  The namespace `argocd` is the default namespace for an ArgoCD instance.

## ArgoCD CLI

### Prerequisites

- Download and install the ArgoCD CLI from the official ArgoCD repository
or from your package manager (e.g. `brew install argocd`).

### Usage

1. Ensure that you are logged in to your ArgoCD server using the following command:

        argocd login <ARGOCD_SERVER_URL> --grpc-web --sso

    Replace `<ARGOCD_SERVER_URL>` with the URL of your ArgoCD server.

2. After login you can use the CLI to add/create/update/view apps and appsets:

    For example:

        argocd app list -o name

    and

        argocd appset list -o name

    will list all apps and appsets in your ArgoCD server.

3. To create/update your apps or appsets use the following commands:

        argocd app create -f <APP_YAML> --upsert

    or

        argocd appset create -f <APPSET_YAML> --upsert

    Note: The `--upsert` flag will update the app or appset if it already exists
    (otherwise it will create a new one).

4. To delete an app or appset, use the following commands:

        argocd app delete <APP_NAME>

    or

        argocd appset delete <APPSET_NAME>

    Note: The `--cascade` flag will delete all resources associated with the app
    or appset.

There are various other commands available for the ArgoCD CLI.
For more information, refer to the official ArgoCD documentation.
