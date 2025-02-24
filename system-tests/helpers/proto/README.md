# Protobuf Files

The `service.proto` file in this folder was copied from
`proto/sequencerblockapis/astria/sequencerblock/v1/service.proto` and stripped
down to simplify executing the protoc compiler.

The files in the generated folder were created following instructions in
[the gRPC Python Basics tutorial](https://grpc.io/docs/languages/python/basics)
using the following commands:

```shell
pip3 install grpcio-tools
python3 -m grpc_tools.protoc \
  -I helpers/proto/generated=system-tests/helpers/proto \
  -I $(python3 -c "import site; print(site.getsitepackages()[0])")
  --python_out=system-tests \
  --pyi_out=system-tests \
  --grpc_python_out=system-tests \
  system-tests/helpers/proto/service.proto
```
