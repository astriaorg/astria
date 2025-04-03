import grpc
import requests
from .utils import (
    check_change_infos,
    run_subprocess,
    wait_for_statefulset_rollout,
    Retryer,
)
from .generated.astria.sequencerblock.v1.service_pb2 import GetSequencerBlockRequest, GetUpgradesInfoRequest
from .generated.astria.sequencerblock.v1.service_pb2_grpc import SequencerServiceStub

class SequencerController:
    """
    A controller targeting a single sequencer node.

    It provides methods for starting and upgrading the node and accessing the node's RPC and gRPC
    servers.
    """

    def __init__(self, node_name):
        self.name = node_name
        if node_name == "node0":
            self.namespace = "astria-dev-cluster"
            self.rpc_url = "http://rpc.sequencer.localdev.me"
            self.grpc_url = "grpc.sequencer.localdev.me:80"
        else:
            self.namespace = f"astria-validator-{node_name}"
            self.rpc_url = f"http://rpc.sequencer-{node_name}.localdev.me"
            self.grpc_url = f"grpc.sequencer-{node_name}.localdev.me:80"
        self.last_block_height_before_restart = None

    # ===========================================================
    # Methods managing and querying the sequencer's k8s container
    # ===========================================================

    def deploy_sequencer(
            self,
            sequencer_image_tag,
            relayer_image_tag,
            enable_price_feed=True,
            upgrade_name=None,
            upgrade_activation_height=None,
    ):
        """
        Deploys a new sequencer on the cluster using the specified image tag.

        The sequencer (and associated sequencer-relayer) are installed via `helm install`, then
        when the rollout has completed.

        If `upgrade_name` is set, then the chart value `upgradeTest` will be set to true to ensure
        genesis.json and upgrades.json have appropriate values set for pre-upgrade.

        In this case, whether the actual upgrade details for this upgrade are included in
        upgrades.json depends upon setting `upgrade_activation_height`. `None` means they are
        omitted (used when starting nodes before the upgrade has activated).
        """
        args = self._helm_args(
            "install",
            sequencer_image_tag,
            relayer_image_tag,
            enable_price_feed,
            upgrade_name,
            upgrade_activation_height
        )
        run_subprocess(args, msg=f"deploying {self.name}")
        self._wait_for_deploy(timeout_secs=600)
        self._ensure_reported_name_matches_assigned_name()
        print(f"{self.name}: running")

    def stage_upgrade(self, sequencer_image_tag, relayer_image_tag, enable_price_feed, upgrade_name, activation_height):
        """
        Updates the sequencer and sequencer-relayer in the cluster.

        This method simply stages the upgrade; it doesn't wait for the binaries to restart.
        """
        try:
            # If we can fetch the latest block height, this is likely due to the sequencer having
            # crashed after missing the upgrade activation.
            self.last_block_height_before_restart = self.try_get_last_block_height()
        except:
            self.last_block_height_before_restart = None

        # Update the upgrades.json file with the specified activation height and upgrade the images
        # for sequencer and sequencer-relayer.
        args = self._helm_args(
            "upgrade",
            sequencer_image_tag,
            relayer_image_tag,
            enable_price_feed=enable_price_feed,
            upgrade_name=upgrade_name,
            upgrade_activation_height=activation_height
        )

        run_subprocess(args, msg=f"upgrading {self.name}")
        if not self.last_block_height_before_restart:
            # In this case, the sequencer process has stopped. Try restarting the pod.
            run_subprocess(
                ["kubectl", "delete", "pod", f"-n={self.namespace}", "sequencer-0"],
                msg=f"restarting pod for {self.name}"
            )

    def wait_for_upgrade(self, upgrade_activation_height):
        """
        Waits for the sequencer to start following staging an upgrade and for it to execute the
        upgrade.

        Expected to be called after calling `stage_upgrade`.
        """
        # Allow 30s for termination, and a further 30s for deployment.
        self._wait_for_deploy(timeout_secs=60)

        # Wait for the sequencer to restart and commit two blocks after the last block recorded
        # before restarting.
        # NOTE: Two blocks rather than just one in case a new block was added in the small window
        #       between fetching the latest block height and actually shutting down.
        # NOTE: If `last_block_height_before_restart` is `None`, this node crashed rather than
        #       being killed for upgrade. This would happen if e.g. the node's binary wasn't
        #       replaced before the upgrade activation point. In this case, just skip the checks
        #       for scheduled upgrade change infos.
        if self.last_block_height_before_restart:
            self.wait_until_chain_at_height(
                self.last_block_height_before_restart + 2,
                timeout_secs=60
            )
            # Fetch and check the upgrade change infos. Ensure we're at least a few blocks before
            # the upgrade activation point, so we can safely expect there should be some changes
            # scheduled and none applied.
            latest_block_height = self.get_last_block_height()
            if latest_block_height < upgrade_activation_height - 2:
                applied, scheduled = self.get_upgrades_info()
                if len(list(applied)) != 0:
                    raise SystemExit(
                        f"{self.name} upgrade error: should have 0 applied upgrade change infos"
                    )
                check_change_infos(scheduled, upgrade_activation_height)
                for change_info in scheduled:
                    print(
                        f"{self.name}: scheduled change info: [{change_info.change_name}, "
                        f"activation_height: {change_info.activation_height}, app_version: "
                        f"{change_info.app_version}, change_hash: {change_info.base64_hash}]",
                        flush=True
                    )
            timeout_secs = max((upgrade_activation_height - latest_block_height) * 10, 30)
        else:
            timeout_secs = upgrade_activation_height * 10
        # Wait for the sequencer to reach the activation point, meaning it should have executed
        # the upgrade.
        self.wait_until_chain_at_height(upgrade_activation_height, timeout_secs)

    # ===========================================
    # Methods calling sequencer's JSON-RPC server
    # ===========================================

    def get_last_block_height(self):
        """
        Queries the sequencer's JSON-RPC server for the latest block height.

        Exits the process on error.
        """
        try:
            response = self._try_send_json_rpc_request_with_retry("abci_info")
            return int(response["response"]["last_block_height"])
        except Exception as error:
            raise SystemExit(f"{self.name}: failed to get last block height: {error}")

    def try_get_last_block_height(self):
        """
        Tries once only to query the sequencer's JSON-RPC server for the latest block height.

        Throws a `requests` exception on error.
        """
        response = self._try_send_json_rpc_request("abci_info")
        return int(response["response"]["last_block_height"])

    def get_vote_extensions_enable_height(self):
        """
        Queries the sequencer's JSON-RPC server for `vote_extensions_enable_height` ABCI consensus
        parameter.

        Exits the process on error.
        """
        # NOTE: This RPC is flaky when no height is specified and often responds with e.g.
        # `{'code': -32603, 'message': 'Internal error', 'data': 'could not find consensus params
        # for height #123: value retrieved from db is empty'}`. Get the latest block height to pass
        # as an arg.
        height = self.get_last_block_height()
        response = self._try_send_json_rpc_request_with_retry(
            "consensus_params", ("height", str(height))
        )
        return int(response["consensus_params"]["abci"]["vote_extensions_enable_height"])

    def wait_until_chain_at_height(self, height, timeout_secs):
        """
        Polls the sequencer's JSON-RPC server for the latest block height until the given height is
        reached or exceeded.

        Exits the process if this condition is not achieved within `timeout_secs` seconds.
        """
        retryer = Retryer(timeout_secs, initial_delay_secs=1, exponential_backoff=False)
        latest_block_height = None
        while True:
            try:
                latest_block_height = self.try_get_last_block_height()
                if latest_block_height >= height:
                    break
            except Exception as error:
                print(f"{self.name}: failed to get latest block height: {error}", flush=True)
                pass
            try:
                retryer.wait()
            except RuntimeError:
                raise SystemExit(
                    f"{self.name} failed to reach block {height} within {timeout_secs} "
                    f"seconds. Latest block height: {latest_block_height}"
                )
            print(
                f"{self.name}: latest block height: {latest_block_height}, awaiting block {height}"
                f", {retryer.seconds_remaining():.3f} seconds remaining",
                flush=True
            )
        print(f"{self.name}: latest block height: {latest_block_height}, finished awaiting block {height}")

    def get_app_version_at_genesis(self):
        """
        Queries the sequencer's JSON-RPC server for the app version as reported via the `genesis`
        method.

        Exits the process on error.
        """
        try:
            response = self._try_send_json_rpc_request_with_retry("genesis")
            return int(response["genesis"]["consensus_params"]["version"]["app"])
        except Exception as error:
            raise SystemExit(f"{self.name}: failed to get current app version: {error}")

    def get_current_app_version(self):
        """
        Queries the sequencer's JSON-RPC server for the current app version as reported via the
        `abci_info` method.

        Exits the process on error.
        """
        try:
            response = self._try_send_json_rpc_request_with_retry("abci_info")
            return int(response["response"]["app_version"])
        except Exception as error:
            raise SystemExit(f"{self.name}: failed to get current app version: {error}")

    # =======================================
    # Methods calling sequencer's gRPC server
    # =======================================

    def get_sequencer_block(self, height):
        """
        Queries the sequencer's gRPC server for the sequencer block at the given height.

        Exits the process on error or timeout.
        """
        try:
            return self._try_send_grpc_request_with_retry(GetSequencerBlockRequest(height=height))
        except Exception as error:
            raise SystemExit(f"{self.name}: failed to get sequencer block {height}:\n{error}\n")

    def get_upgrades_info(self):
        """
        Queries the sequencer's gRPC server for the upgrades info.

        Exits the process on error or timeout.
        """
        try:
            response = self._try_send_grpc_request_with_retry(GetUpgradesInfoRequest())
            return response.applied, response.scheduled
        except Exception as error:
            raise SystemExit(f"{self.name}: failed to get upgrade info:\n{error}\n")

    # ===============
    # Private methods
    # ===============

    def _helm_args(
            self,
            subcommand,
            sequencer_image_tag,
            relayer_image_tag,
            enable_price_feed,
            upgrade_name,
            upgrade_activation_height,
    ):
        args = [
            "helm",
            subcommand,
            f"-n={self.namespace}",
            f"{self.name}-sequencer-chart",
            "charts/sequencer",
            "--values=dev/values/validators/all.yml",
            f"--values=dev/values/validators/{self.name}.yml",
            f"--set=images.sequencer.tag={sequencer_image_tag}",
            f"--set=sequencer-relayer.images.sequencerRelayer.tag={relayer_image_tag}",
            f"--set=sequencer.priceFeed.enabled={enable_price_feed}",
            "--set=sequencer.abciUDS=false",
        ]
        if subcommand == "install":
            args.append("--create-namespace")
        if upgrade_name:
            # This is an upgrade test: set `upgradeTest` so as to provide an upgrades.json file
            # and genesis.json without upgraded configs.  Also enable persistent storage.
            args.append("--set=storage.enabled=true")
            args.append("--set=sequencer-relayer.storage.enabled=true")
            args.append(f"--values=dev/values/validators/{upgrade_name}.upgrades.yml")
            # If we know the activation height of the upgrade, add it to the relevant upgrade's
            # settings for inclusion in the upgrades.json file.
            if upgrade_activation_height:
                args.append(
                    f"--set=upgrades.{upgrade_name}.baseInfo.activationHeight={upgrade_activation_height}"
                )
            else:
                # Otherwise, if no activation height is provided, we will simply omit the upgrade
                # details from the upgrades.json file.
                args.append(f"--set=upgrades.{upgrade_name}.enabled=false")
        elif enable_price_feed:
            # If we're not upgrading, and the price feed is enabled, enable it in the
            # genesis.json file.
            args.append("--values=dev/values/validators/priceFeed.genesis.yml")
        return args

    def _wait_for_deploy(self, timeout_secs):
        wait_for_statefulset_rollout(self.name, "sequencer", self.namespace, timeout_secs)

    def _try_send_json_rpc_request_with_retry(self, method, *params, timeout_secs=30):
        """
        Sends a JSON-RPC request to the associated sequencer's RPC server with the given method and
        params.

        `params` should be pairs of key-value strings.

        Retries with an exponential backoff between attempts for up to `timeout_secs` seconds.

        Throws a `requests` exception if all the RPC calls fail, or a `RuntimeError` if a JSON-RPC
        response is an error.
        """
        retryer = Retryer(timeout_secs)
        while True:
            try:
                return self._try_send_json_rpc_request(method, *params)
            except Exception as error:
                print(f"{self.name}: rpc failed: {error}, retrying in {retryer.sleep_secs:.3f} seconds")
            try:
                retryer.wait()
            except RuntimeError:
                raise RuntimeError(f"rpc failed {retryer.successful_wait_count + 1} times, giving up")

    def _try_send_json_rpc_request(self, method, *params):
        """
        Sends a single JSON-RPC request (i.e. no retries) to the associated sequencer's RPC
        server with the given method and params.

        `params` should be pairs of key-value strings.

        Throws a `requests` exception if the RPC call fails, or a `RuntimeError` if the JSON-RPC
        response is an error.
        """
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": dict(params),
            "id": 1,
        }
        response = requests.post(self.rpc_url, json=payload)
        if response.status_code != 200:
            raise RuntimeError(f"json-rpc response for `{payload}`: code {response.status_code}")
        json_response = response.json()
        if not "result" in json_response:
            raise RuntimeError(f"json-rpc error response for `{payload}`: {json_response['error']}")
        return json_response["result"]

    def _try_send_grpc_request_with_retry(self, request, timeout_secs=10):
        """
        Sends a gRPC request to the associated sequencer's gRPC server.

        Retries with an exponential backoff between attempts for up to `timeout_secs` seconds.

        Throws an exception if all the gRPC calls fail.
        """
        retryer = Retryer(timeout_secs)
        while True:
            try:
                return self._try_send_grpc_request(request)
            except Exception as error:
                print(f"{self.name}: grpc failed: {error}, retrying in {retryer.sleep_secs:.3f} seconds")
            try:
                retryer.wait()
            except RuntimeError:
                raise RuntimeError(f"grpc failed {retryer.successful_wait_count + 1} times, giving up")

    def _try_send_grpc_request(self, request):
        """
        Sends a single gRPC request (i.e. no retries) to the associated sequencer's gRPC server.

        Throws an exception if the gRPC call fails.
        """
        channel = grpc.insecure_channel(self.grpc_url)
        grpc_client = SequencerServiceStub(channel)
        if isinstance(request, GetSequencerBlockRequest):
            return grpc_client.GetSequencerBlock(request)
        elif isinstance(request, GetUpgradesInfoRequest):
            return grpc_client.GetUpgradesInfo(request)
        else:
            raise SystemExit(
                f"failed to send grpc request: {str(request).strip()} is an unknown type"
            )

    def _ensure_reported_name_matches_assigned_name(self):
        """
        Ensures the node name provided in `__init__` matches the moniker of the node we're
        associated with.
        """
        try:
            response = self._try_send_json_rpc_request_with_retry("status", timeout_secs=600)
            reported_name = response["node_info"]["moniker"]
            if reported_name == self.name:
                return
            else:
                raise SystemExit(
                    f"provided name `{self.name}` does not match moniker `{reported_name}` as "
                    "reported in `status` json-rpc response"
                )
        except Exception as error:
            raise SystemExit(
                f"{self.name}: failed to fetch node name: {error}"
            )
