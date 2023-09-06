# Celestia JSON RPC client

This crate provides a high level API to interact with the Celestia JSON RPC.

## Testing

The API is checked against an instance of `celestia-node:v0.11.0-rc7`. For local
testing, a kubernetes cluståer must be present and running. From the root of the
monorepo:

```sh
kind create --cluster --config kubernetes-ci/cluster-config.yml
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
kubectl apply -k crates/astria-celestia-jsonrpc-client/k8s/
cargo test -p astria-celestia-jsonrpc-client -- --ignored
```
