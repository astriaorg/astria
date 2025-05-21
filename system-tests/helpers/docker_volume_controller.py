from .utils import try_run_subprocess, run_subprocess

class DockerVolumeController:
    """
    A class to manage docker volumes.
    """

    def __init__(self, volume_name):
        """
        Initialize the DockerVolumeController with the specified volume name.
        """
        self.volume_name = volume_name
        run_subprocess(
            ["docker", "volume", "create", self.volume_name],
            msg=f"creating docker volume `{self.volume_name}`"
        )

    def __del__(self):
        """
        Clean up the docker volume when the object is deleted.
        """
        try:
            try_run_subprocess(
                ["docker", "volume", "remove", self.volume_name],
                msg=f"removing docker volume `{self.volume_name}`"
            )
        except Exception as e:
            print(f"Error cleaning up docker volume `{self.volume_name}`: {e}")

    def map_and_execute_data(self, args):
        """
        Maps the docker volume to the `data` directory and executes a command
        in the context of the docker volume.
        """
        run_subprocess(
            ["docker", "run", "--rm", "-v", f"{self.volume_name}:/data", *args],
            msg=f"executing command in docker volume `{self.volume_name}`"
        )

    def map_and_execute_astria(self, args):
        """
        Maps the docker volume to the `/astria` directory and executes a command
        in the context of the docker volume with the Astria CLI.
        """
        run_subprocess(
            ["docker", "run", "--rm", "-v", f"{self.volume_name}:/astria", *args],
            msg=f"executing command in docker volume `{self.volume_name}` with Astria CLI"
        )
