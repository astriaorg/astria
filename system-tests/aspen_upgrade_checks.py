"""
This module contains checks specific to `Aspen`.
"""

def assert_pre_upgrade_conditions(nodes):
    _check_vote_extensions_enable_height(nodes, 0)
    _check_extended_commit_info_in_sequencer_block(nodes, height=2, should_be_present=False)

def assert_post_upgrade_conditions(nodes, upgrade_activation_height):
    _check_vote_extensions_enable_height(nodes, upgrade_activation_height + 1)
    _check_extended_commit_info_in_sequencer_block(
        nodes,
        height=upgrade_activation_height + 1,
        should_be_present=False
    )
    _check_extended_commit_info_in_sequencer_block(
        nodes,
        height=upgrade_activation_height + 2,
        should_be_present=True
    )

def _check_vote_extensions_enable_height(nodes, expected):
    for node in nodes:
        actual = node.get_vote_extensions_enable_height()
        if actual != expected:
            raise SystemExit(
                f"{node.name}: `vote_extensions_enable_height` of {actual}, expected {expected}"
            )

def _check_extended_commit_info_in_sequencer_block(nodes, height, should_be_present):
    for node in nodes:
        node.wait_until_chain_at_height(height, 10)
        block = node.get_sequencer_block(height)
        is_present = bool(block.extended_commit_info_with_proof.extended_commit_info)
        if is_present and not should_be_present:
            raise SystemExit(
                f"{node.name}: block {height} contained unexpected extended commit info"
            )
        if not is_present and should_be_present:
            raise SystemExit(
                f"{node.name}: block {height} did not contain extended commit info"
            )
