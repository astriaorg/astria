import requests
from .defaults import EVM_DESTINATION_ADDRESS
from .utils import run_subprocess, wait_for_statefulset_rollout, Retryer

class EvmController:
    """
    A controller targeting the EVM rollup processes.

    It provides methods for starting and upgrading the rollup and accessing the EVM node's RPC and
    websocket servers.
    """

    # ========================================================
    # Methods managing and querying the rollup's k8s container
    # ========================================================

    def deploy_rollup(self, image_controller, evm_restart=False):
        """
        Deploys a new EVM rollup on the cluster using the specified image tags.

        The EVM node, composer, conductor and bridge-withdrawer are installed via `helm install`.
        """
        run_subprocess(self._helm_args("install", image_controller, evm_restart=evm_restart), msg="deploying rollup")
        wait_for_statefulset_rollout("rollup", "astria-geth", "astria-dev-cluster", 600)

    # ========================================
    # Methods calling rollup's JSON-RPC server
    # ========================================

    def get_balance(self, address=EVM_DESTINATION_ADDRESS):
        """
        Queries the rollup's JSON-RPC server for the balance of the given account.

        Exits the process on error.
        """
        try:
            hex_balance = self._try_send_json_rpc_request("eth_getBalance", address, "latest")
            return int(hex_balance, 16)
        except Exception as error:
            raise SystemExit(f"rollup: failed to get balance for {address}: {error}")

    def wait_until_balance(self, address, expected_balance, timeout_secs):
        """
        Polls the rollup's JSON-RPC server for the balance of the given account until the expected
        balance is reached.

        Exits the process if this condition is not achieved within `timeout_secs` seconds.
        """
        retryer = Retryer(timeout_secs, initial_delay_secs=1, exponential_backoff=False)
        while True:
            balance = self.get_balance(address)
            if balance == expected_balance:
                break
            try:
                retryer.wait()
            except RuntimeError:
                raise SystemExit(
                    f"failed to get evm balance {expected_balance} within {timeout_secs} "
                    f"seconds. Current evm balance: {balance}"
                )
            print(
                f"current evm balance: {balance}, awaiting evm balance of {expected_balance}, "
                f"{retryer.seconds_remaining():.3f} seconds remaining"
            )
        print(f"current evm balance: {balance}, finished waiting")

    def wait_until_chain_at_height(self, height, timeout_secs):
        """
        Polls the rollup's JSON-RPC server for the latest finalized block height until the given
        height is reached or exceeded.

        Exits the process if this condition is not achieved within `timeout_secs` seconds.
        """
        retryer = Retryer(timeout_secs, initial_delay_secs=1, exponential_backoff=False)
        latest_block_height = None
        while True:
            try:
                response = self._try_send_json_rpc_request("eth_getBlockByNumber", "finalized", False)
                latest_block_height = int(response["number"], 16)
            except Exception as error:
                print(
                    f"rollup: failed to get latest evm finalized block height: "
                    f"{error}\nrollup: retrying",
                )
                pass
            if latest_block_height and latest_block_height >= height:
                break
            try:
                retryer.wait()
            except RuntimeError:
                raise SystemExit(
                    f"rollup failed to reach finalized block {height} within {timeout_secs} "
                    f"seconds. Latest evm finalized block height: {latest_block_height}"
                )
            print(
                f"rollup: latest finalized block height: {latest_block_height}, awaiting block "
                f"{height}, {retryer.seconds_remaining():.3f} seconds remaining",
            )
        print(
            f"rollup: latest finalized block height: {latest_block_height}, finished awaiting "
            f"block {height}"
        )

    def send_raw_tx(self, tx_data):
        """
        Send the raw tx bytes to the rollup's JSON-RPC server.  The bytes should be in the form of a
        hex-encoded string with a prefix of `0x`.

        Exits the process on error.
        """
        try:
            self._try_send_json_rpc_request("eth_sendRawTransaction", tx_data)
        except Exception as error:
            raise SystemExit(f"rollup: failed to send raw tx: {error}")

    def get_tx_block_number(self, tx_hash):
        """
        Queries the rollup's JSON-RPC server for the receipt of the given transaction and returns
        the block number from the receipt.

        Exits the process on error.
        """
        try:
            receipt = self._try_send_json_rpc_request("eth_getTransactionReceipt", tx_hash)
            return int(receipt["blockNumber"], 16)
        except Exception as error:
            raise SystemExit(f"rollup: failed to get tx receipt: {error}")

    # ===============
    # Private methods
    # ===============

    def _helm_args(self, subcommand, image_controller, evm_restart):
        values = "evm-restart-test" if evm_restart else "dev"
        args = [
            "helm",
            subcommand,
            "-n=astria-dev-cluster",
            "astria-chain-chart",
            "charts/evm-stack",
            f"--values=dev/values/rollup/{values}.yaml",
            "--set=blockscout-stack.enabled=false",
            "--set=postgresql.enabled=false",
            "--set=evm-faucet.enabled=false",
        ]
        conductor_image_tag = image_controller.conductor_image_tag()
        if conductor_image_tag is not None:
            args.append(f"--set=evm-rollup.images.conductor.tag={conductor_image_tag}")
        composer_image_tag = image_controller.composer_image_tag()
        if composer_image_tag is not None:
            args.append(f"--set=composer.images.composer.devTag={composer_image_tag}")
        bridge_withdrawer_image_tag = image_controller.bridge_withdrawer_image_tag()
        if bridge_withdrawer_image_tag is not None:
            args.append(f"--set=evm-bridge-withdrawer.images.evmBridgeWithdrawer.devTag={bridge_withdrawer_image_tag}")
        geth_image_tag = image_controller.geth_image_tag()
        if geth_image_tag is not None:
            args.append(f"--set=evm-rollup.images.geth.tag={geth_image_tag}")
        return args

    def _try_send_json_rpc_request(self, method, *params):
        """
        Sends a single JSON-RPC request (i.e. no retries) to the associated rollup's RPC
        server with the given method and params.

        `params` should be a list of alternating key and value strings.

        Throws a `requests` exception if the RPC call fails, or a `RuntimeError` if the JSON-RPC
        response is an error.
        """
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": list(params),
            "id": 1,
        }
        response = requests.post(f"http://executor.astria.127.0.0.1.nip.io/", json=payload).json()
        if not "result" in response:
            raise RuntimeError(f"json-rpc error response for `{method}`: {response['error']}")
        return response["result"]
