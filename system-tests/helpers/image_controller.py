from typing import Dict, List

VALID_COMPONENTS = [
    "all",
    "sequencer",
    "sequencer-relayer",
    "conductor",
    "composer",
    "bridge-withdrawer",
    "geth",
    "cli",
    ]

MONOREPO_COMPONENTS = [
    "sequencer",
    "sequencer-relayer",
    "conductor",
    "composer",
    "bridge-withdrawer",
    "cli",
    ]

class ImageController:
    """
    A class to manage the image controller for a system test.
    """

    def __init__(self, image_tags: List[str]):
        """
        Parses a list of image tag arguments in the format 'component=tag' to be stored in the controller.

        Example:
            ['sequencer=local', 'conductor=latest'] -> {'sequencer': 'local', 'conductor': 'latest'}
        """
        self.image_tags = {}
        for tag_spec in image_tags:
            try:
                try:
                    component, tag = tag_spec.split('=', 1)
                except Exception:
                    raise SystemExit(f"Invalid image tag format: {tag_spec}. Expected format: 'component=tag'")
                if component not in VALID_COMPONENTS:
                    raise SystemExit(f"Invalid component name: `{component}`. Valid components are: {VALID_COMPONENTS}")
                if component == "all":
                    for valid_component in MONOREPO_COMPONENTS:
                        self.image_tags[valid_component] = tag
                else:
                    self.image_tags[component] = tag
            except Exception as error:
                raise SystemExit(error)

    def add_argument(parser):
        """
        Adds an argument to the given parser for image tags.
        """
        parser.add_argument(
            "-i", "--image-tag",
            help=
                "Image tag in the format 'component=tag', e.g. 'sequencer=local'. Can \
                    be specified multiple times. Available components: sequencer, \
                    sequencer-relayer, conductor, composer, bridge-withdrawer, \
                    geth, cli, all. NOTE: 'all' sets all components to the same \
                    tag EXCEPT for 'geth', which exists in a separate repository.",
            metavar="COMPONENT=TAG",
            action="append",
            default=[]
        )

    def sequencer_image_tag(self) -> str:
        """
        Returns the image tag for the sequencer component.
        """
        return self.image_tags.get("sequencer", None)

    def sequencer_relayer_image_tag(self) -> str:
        """
        Returns the image tag for the sequencer relayer component.
        """
        return self.image_tags.get("sequencer-relayer", None)

    def conductor_image_tag(self) -> str:
        """
        Returns the image tag for the conductor component.
        """
        return self.image_tags.get("conductor", None)

    def composer_image_tag(self) -> str:
        """
        Returns the image tag for the composer component.
        """
        return self.image_tags.get("composer", None)

    def bridge_withdrawer_image_tag(self) -> str:
        """
        Returns the image tag for the bridge withdrawer component.
        """
        return self.image_tags.get("bridge-withdrawer", None)

    def geth_image_tag(self) -> str:
        """
        Returns the image tag for the geth component.
        """
        return self.image_tags.get("geth", None)

    def cli_image_tag(self) -> str:
        """
        Returns the image tag for the cli component.
        """
        return self.image_tags.get("cli", None)
