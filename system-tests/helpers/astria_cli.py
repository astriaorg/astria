from python_on_whales import docker
from python_on_whales.exceptions import DockerException
from .utils import Retryer

SEQUENCER_RPC_POD_PORT = 26657

class Cli:
    """
    An instance of the astria-cli.
    """

    def __init__(self, image_tag="latest"):
        self.image_tag = image_tag

    def set_image_tag(self, image_tag):
        self.image_tag = image_tag

    def wait_until_balance(self, account, expected_balance, timeout_secs, sequencer_name):
        """
        Polls for the balance of the given account until the expected balance is reached.

        Exits the process if this condition is not achieved within `timeout_secs` seconds.
        """
        retryer = Retryer(timeout_secs, initial_delay_secs=1, exponential_backoff=False)
        balance = None
        while True:
            try:
                balance = self._try_get_balance(account, sequencer_name)
                if balance == expected_balance:
                    break
            except Exception as error:
                print(f"failed to get balance: {error}")
                pass
            try:
                retryer.wait()
            except RuntimeError:
                raise SystemExit(
                    f"failed to get balance {expected_balance} within {timeout_secs} "
                    f"seconds. Current balance: {balance}"
                )
            print(
                f"current balance: {balance}, awaiting balance of {expected_balance}, "
                f"{retryer.seconds_remaining():.3f} seconds remaining"
            )
        print(f"current balance: {balance}, finished waiting")

    def init_bridge_account(self, sequencer_name):
        try:
            self._try_exec_sequencer_command_with_retry(
                "init-bridge-account",
                "--rollup-name=astria",
                "--private-key=dfa7108e38ab71f89f356c72afc38600d5758f11a8c337164713e4471411d2e0",
                "--sequencer.chain-id=sequencer-test-chain-0",
                "--fee-asset=nria",
                "--asset=nria",
                sequencer_name=sequencer_name
            )
        except Exception as error:
            raise SystemExit(error)

    def bridge_lock(self, sequencer_name):
        try:
            self._try_exec_sequencer_command_with_retry(
                "bridge-lock",
                "astria13ahqz4pjqfmynk9ylrqv4fwe4957x2p0h5782u",
                "--amount=10000000000",
                "--destination-chain-address=0xaC21B97d35Bf75A7dAb16f35b111a50e78A72F30",
                "--private-key=934ab488f9e1900f6a08f50605ce1409ca9d95ebdc400dafc2e8a4306419fd52",
                "--sequencer.chain-id=sequencer-test-chain-0",
                "--fee-asset=nria",
                "--asset=nria",
                sequencer_name=sequencer_name
            )
        except Exception as error:
            raise SystemExit(error)

    def validator_update(self, sequencer_name, sequencer_pub_key, power):
        try:
            self._try_exec_sequencer_command(
                "sudo validator-update",
                "--sequencer.chain-id=sequencer-test-chain-0",
                f"--validator-public-key={sequencer_pub_key}",
                "--private-key=2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90",
                f"--power={power}",
                f"--name={sequencer_name}",
                sequencer_name=sequencer_name,
            )
        except Exception as error:
            raise SystemExit(error)

    def address(self, sequencer_name, address_bytes):
        try:
            return self._try_exec_sequencer_command(
                "address bech32m",
                f"--bytes={address_bytes}",
                "--prefix=astria",
                sequencer_name=sequencer_name,
                use_sequencer_url=False,
            )
        except Exception as error:
            raise SystemExit(error)

    def _try_get_balance(self, account, sequencer_name):
        """
        Tries to get the given account's balance by calling `astria-cli sequencer account balance`.
        """
        stdout = self._try_exec_sequencer_command_with_retry(
            "account", "balance", account, sequencer_name=sequencer_name
        )
        balance_line = stdout.splitlines().pop()
        if balance_line.endswith("nria"):
            return int(balance_line[:-4])
        else:
            raise RuntimeError(
                "expected last line of cli `sequencer account balance` output to end with `nria`: "
                f"stdout: `{stdout}`"
            )

    def _try_exec_sequencer_command_with_retry(self, *args, sequencer_name, timeout_secs=10):
        """
        Tries to execute the CLI `sequencer` subcommand via `docker run`.

        `sequencer` and `--sequencer-url` should NOT be passed in the `args`; they will be added in
        this method based upon the value of `sequencer_name`.

        Retries with an exponential backoff between attempts for up to `timeout_secs` seconds.
        """
        retryer = Retryer(timeout_secs)
        while True:
            try:
                return self._try_exec_sequencer_command(*args, sequencer_name=sequencer_name)
            except Exception as error:
                last_error = error
                print(f"cli: rpc failed, retrying in {retryer.sleep_secs:.3f} seconds")
            try:
                retryer.wait()
            except RuntimeError:
                raise RuntimeError(
                    f"{last_error}\nrpc failed {retryer.successful_wait_count + 1} times, giving up"
                )

    def _try_exec_sequencer_command(self, *args, sequencer_name, use_sequencer_url=True):
        """
        Tries once (i.e. no retries) to execute the CLI `sequencer` subcommand via `docker run`.

        `sequencer` and `--sequencer-url` should NOT be passed in the `args`; they will be added in
        this method based upon the value of `sequencer_name`.

        Returns the stdout output on success, or throws a `DockerException` otherwise.
        """
        if sequencer_name == "node0":
            url = "http://rpc.sequencer.localdev.me"
        else:
            url = f"http://rpc.sequencer-{sequencer_name}.localdev.me"
        args = list(args)
        args.insert(0, "sequencer")
        if use_sequencer_url:
            args.append(f"--sequencer-url={url}")
        print(
            f"cli: running `docker run --rm --network "
            f"host ghcr.io/astriaorg/astria-cli:{self.image_tag} {' '.join(map(str, args))}`"
        )
        try:
            return docker.run(
                f"ghcr.io/astriaorg/astria-cli:{self.image_tag}",
                args,
                networks=["host"],
                remove=True,
            )
        except DockerException as error:
            print(f"Exit code {error.return_code} while running {error.docker_command}")
            raise error
