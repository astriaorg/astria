from .utils import run_subprocess, wait_for_deployment

HERMES_REPO_URL = "ghcr.io/astriaorg/hermes:"

class HermesController:
    def __init__(self, name):
        self.name = name

    def deploy_hermes(self, image_controller):
        """
        Deploys a Hermes node.
        """
        run_subprocess(self._helm_args("install", image_controller), msg="deploying hermes")
        wait_for_deployment(f"hermes-{self.name}-chart", f"hermes-{self.name}", 600)

    def check_logs(self, to_check):
        """
        Checks the logs of the Hermes node for the specified string, returning
        True if found, False otherwise.
        """
        args = [
            "kubectl",
            "logs",
            f"-n=hermes-{self.name}",
            f"deployment/hermes-{self.name}-chart",
        ]
        result = run_subprocess(args, msg="getting hermes logs")
        return to_check in result.stdout.decode("utf-8")


    # ===============
    # Private methods
    # ===============

    def _helm_args(self, subcommand, image_controller):
        args = [
            "helm",
            subcommand,
            f"-n=hermes-{self.name}",
            f"hermes-{self.name}-chart",
            "charts/hermes",
            f"--values=dev/values/hermes/{self.name}.yaml",
            "--create-namespace",
        ]
        image_tag = image_controller.hermes_image_tag()
        if image_tag is not None:
            args.append(f"--set image={HERMES_REPO_URL}{image_tag}")
        return args
