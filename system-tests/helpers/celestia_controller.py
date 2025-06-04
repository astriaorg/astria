from .utils import (
    run_subprocess,
    wait_for_statefulset_rollout,
)
from .defaults import (
    EVM_DESTINATION_ADDRESS,
    IBC_TRANSFER_AMOUNT,
)

BRIDGE_ADDRESS = ""
CELESTIA_DEV_ACCOUNT_ADDRESS = "celestia1m0ksdjl2p5nzhqy3p47fksv52at3ln885xvl96"
CELESTIA_CHAIN_ID = "celestia-local-0"
KEYRING_BACKEND = "test"
IBC_TRANSFER_FEES = "26000"

class CelestiaController:
    def deploy_celestia(self):
        """
        Deploys a Celestia node.
        """
        run_subprocess(self._helm_args("install"), msg="deploying celestia local")
        wait_for_statefulset_rollout("celestia-local", "celestia-local", "astria-dev-cluster", 600)

    def do_ibc_transfer(self, to_address):
        """
        Initiates an IBC transfer to the specified address.
        """
        run_subprocess(self._kubectl_transfer_args(to_address), msg=f"transferring to {to_address}")

    # ===============
    # Private methods
    # ===============

    def _helm_args(self, subcommand):
        return [
            "helm",
            subcommand,
            "-n=astria-dev-cluster",
            "celestia-local-chart",
            "charts/celestia-local",
            "--create-namespace",
        ]

    def _kubectl_transfer_args(self, to_address=BRIDGE_ADDRESS):
        memo_arg = "" if to_address != BRIDGE_ADDRESS else f"--memo=\"{{\"rollupDepositAddress\":\"{EVM_DESTINATION_ADDRESS}\"}}\""
        return [
            "kubectl",
            "exec",
            "-n=astria-dev-cluster",
            "pods/celestia-local-0",
            "celestia-app",
            "--",
            "/bin/bash",
            "-c",
            f'celestia-appd tx ibc-transfer transfer \
                transfer \
                channel-0 \
                {to_address} \
                "{IBC_TRANSFER_AMOUNT}utia" \
                {memo_arg} \
                --chain-id="{CELESTIA_CHAIN_ID}" \
                --from="{CELESTIA_DEV_ACCOUNT_ADDRESS}" \
                --fees="{IBC_TRANSFER_FEES}utia" \
                --yes \
                --log_level=debug \
                --home /home/celestia \
                --keyring-backend="{KEYRING_BACKEND}"'
        ]
