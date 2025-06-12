import subprocess
import time
from .defaults import UPGRADE_CHANGES

def run_subprocess(args, msg):
    """
    Runs the provided args as a subprocess.

    `msg` will be printed along with the command being run, and also on failure of the subprocess.
    It should be of the form e.g. "upgrading node1".

    On error, exits the top-level process.
    """
    try:
        return try_run_subprocess(args, msg)
    except RuntimeError as error:
        raise SystemExit(error)

def try_run_subprocess(args, msg):
    """
    Tries to run the provided args as a subprocess.

    `msg` will be printed along with the command being run. It should be of the form e.g.
    "upgrading node1".

    On error, raises a `RuntimeError` exception.
    """
    prefix = f"{msg}: " if msg else ""
    print(f"{prefix}running `{' '.join(map(str, args))}`")
    try:
        return subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=True)
    except subprocess.CalledProcessError as error:
        prefix = f" {msg}: " if msg else ": "
        raise RuntimeError(f"failed{prefix}{error.stdout.decode('utf-8').strip()}")

def wait_for_statefulset_rollout(deploy_name, statefulset_name, namespace, timeout_secs):
    args = [
        "kubectl", "rollout", "status", f"statefulset/{statefulset_name}", f"-n={namespace}",
        f"--timeout={timeout_secs}s"
    ]
    try:
        try_run_subprocess(args, f"waiting for {deploy_name} to deploy")
        return
    except RuntimeError as error:
        print(error)
    # Waiting failed.  Print potentially useful info.
    subprocess.run(["kubectl", "get", "pods", f"-n={namespace}"])
    print()
    subprocess.run(["kubectl", "events", f"-n={namespace}", "--types=Warning"])
    print()
    raise SystemExit(f"failed to deploy {deploy_name} within {timeout_secs} seconds")

def wait_for_deployment(deployment_name, namespace, timeout_secs):
    args = [
        "kubectl", "wait", "deployment", f"{deployment_name}", f"-n={namespace}",
        "--for=condition=Available=True", f"--timeout={timeout_secs}s"
    ]
    try:
        try_run_subprocess(args, f"waiting for {deployment_name} to deploy")
        return
    except RuntimeError as error:
        print(error)
    # Waiting failed.  Print potentially useful info.
    subprocess.run(["kubectl", "get", "pods", f"-n={namespace}"])
    print()
    subprocess.run(["kubectl", "events", f"-n={namespace}", "--types=Warning"])
    print()
    raise SystemExit(f"failed to deploy {deployment_name} within {timeout_secs} seconds")

def update_chart_dependencies(chart):
    args = ["helm", "dependency", "update", f"charts/{chart}"]
    run_subprocess(args, msg=f"updating chart dependencies for {chart}")

def check_change_infos(change_infos, upgrade_heights, expected_app_version=None):
    """
    Assert that the provided change info collection is not empty, and that each entry has the
    expected activation height and app version.

    Exits the process on failure.
    """
    if len(list(change_infos)) == 0:
        raise SystemExit("sequencer upgrade error: no upgrade change info reported")
    for change_info in change_infos:
        # Ascertain the upgrade name from the change name so we can get its expected
        # activation height.
        expected_upgrade_name = None
        for upgrade_name, upgrade_changes in UPGRADE_CHANGES.items():
            if change_info.change_name in upgrade_changes:
                expected_upgrade_name = upgrade_name
                break
        if expected_upgrade_name is None:
            raise SystemExit(
                f"sequencer upgrade error: reported change info has unexpected change name \
                    {change_info.change_name}: expected one of {list(UPGRADE_CHANGES.keys())}"
            )
        expected_activation_height = upgrade_heights[expected_upgrade_name]

        # Check activation height
        if change_info.activation_height != expected_activation_height:
            raise SystemExit(
                "sequencer upgrade error: reported change info does not have expected activation "
                f"height of {expected_activation_height}: reported change info:\n{change_info}"
            )

    # Only want to check the app version of the most recent change info, since
    # previous upgrades will also be included.
    if expected_app_version and change_infos[-1].app_version != expected_app_version:
        raise SystemExit(
            "sequencer upgrade error: reported change info does not have expected app version "
            f"of {expected_app_version}: reported change info:\n{change_infos[-1]}"
        )

class Retryer:
    """
    A helper to support repeatedly sleeping in a loop to allow for a delay between re-attempts.
    """
    def __init__(self, timeout_secs, initial_delay_secs=0.4, exponential_backoff=True):
        """
        :param timeout_secs: The maximum amount of time to allow for all re-attempts. The timer
        starts when `__init__` is called.
        :param initial_delay_secs: The amount of time in seconds to sleep during the first call to
        `self.wait()`. This will be the same for all `wait` calls if `exponential_backoff` is False.
        :param exponential_backoff: If True, the delay in each call to `wait` will be double that of
        the previous call's, up to a maximum value of 5 seconds. If False, the delay in each `wait`
        will be `initial_delay_secs`.
        """
        if timeout_secs <= initial_delay_secs:
            raise ValueError("`timeout_secs` must be greater than `initial_delay_secs`")
        if initial_delay_secs <= 0:
            raise ValueError("`initial_delay_secs` must be greater than zero")
        self.timeout_instant = time.monotonic() + timeout_secs
        self.sleep_secs = initial_delay_secs
        self.exponential_backoff = exponential_backoff
        self.successful_wait_count = 0

    def wait(self):
        """
        Blocks until the next scheduled attempt is due.

        Throws a `RuntimeException` if the timeout would be exceeded during this wait.
        """
        now = time.monotonic()
        if now + self.sleep_secs > self.timeout_instant:
            raise RuntimeError("timed out")
        time.sleep(self.sleep_secs)
        if self.exponential_backoff:
            self.sleep_secs = min(self.sleep_secs * 2, 5)
        self.successful_wait_count += 1

    def seconds_remaining(self):
        return self.timeout_instant - time.monotonic()
