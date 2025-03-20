from google.protobuf import timestamp_pb2 as _timestamp_pb2
from google.protobuf.internal import containers as _containers
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Iterable as _Iterable, Mapping as _Mapping, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class GetSequencerBlockRequest(_message.Message):
    __slots__ = ("height",)
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    height: int
    def __init__(self, height: _Optional[int] = ...) -> None: ...

class RollupId(_message.Message):
    __slots__ = ("inner",)
    INNER_FIELD_NUMBER: _ClassVar[int]
    inner: bytes
    def __init__(self, inner: _Optional[bytes] = ...) -> None: ...

class Proof(_message.Message):
    __slots__ = ("audit_path", "leaf_index", "tree_size")
    AUDIT_PATH_FIELD_NUMBER: _ClassVar[int]
    LEAF_INDEX_FIELD_NUMBER: _ClassVar[int]
    TREE_SIZE_FIELD_NUMBER: _ClassVar[int]
    audit_path: bytes
    leaf_index: int
    tree_size: int
    def __init__(self, audit_path: _Optional[bytes] = ..., leaf_index: _Optional[int] = ..., tree_size: _Optional[int] = ...) -> None: ...

class RollupTransactions(_message.Message):
    __slots__ = ("rollup_id", "transactions", "proof")
    ROLLUP_ID_FIELD_NUMBER: _ClassVar[int]
    TRANSACTIONS_FIELD_NUMBER: _ClassVar[int]
    PROOF_FIELD_NUMBER: _ClassVar[int]
    rollup_id: RollupId
    transactions: _containers.RepeatedScalarFieldContainer[bytes]
    proof: Proof
    def __init__(self, rollup_id: _Optional[_Union[RollupId, _Mapping]] = ..., transactions: _Optional[_Iterable[bytes]] = ..., proof: _Optional[_Union[Proof, _Mapping]] = ...) -> None: ...

class ExtendedCommitInfoWithProof(_message.Message):
    __slots__ = ("extended_commit_info", "proof")
    EXTENDED_COMMIT_INFO_FIELD_NUMBER: _ClassVar[int]
    PROOF_FIELD_NUMBER: _ClassVar[int]
    extended_commit_info: bytes
    proof: Proof
    def __init__(self, extended_commit_info: _Optional[bytes] = ..., proof: _Optional[_Union[Proof, _Mapping]] = ...) -> None: ...

class SequencerBlock(_message.Message):
    __slots__ = ("header", "rollup_transactions", "rollup_transactions_proof", "rollup_ids_proof", "block_hash", "upgrade_change_hashes", "extended_commit_info_with_proof")
    HEADER_FIELD_NUMBER: _ClassVar[int]
    ROLLUP_TRANSACTIONS_FIELD_NUMBER: _ClassVar[int]
    ROLLUP_TRANSACTIONS_PROOF_FIELD_NUMBER: _ClassVar[int]
    ROLLUP_IDS_PROOF_FIELD_NUMBER: _ClassVar[int]
    BLOCK_HASH_FIELD_NUMBER: _ClassVar[int]
    UPGRADE_CHANGE_HASHES_FIELD_NUMBER: _ClassVar[int]
    EXTENDED_COMMIT_INFO_WITH_PROOF_FIELD_NUMBER: _ClassVar[int]
    header: SequencerBlockHeader
    rollup_transactions: _containers.RepeatedCompositeFieldContainer[RollupTransactions]
    rollup_transactions_proof: Proof
    rollup_ids_proof: Proof
    block_hash: bytes
    upgrade_change_hashes: _containers.RepeatedScalarFieldContainer[bytes]
    extended_commit_info_with_proof: ExtendedCommitInfoWithProof
    def __init__(self, header: _Optional[_Union[SequencerBlockHeader, _Mapping]] = ..., rollup_transactions: _Optional[_Iterable[_Union[RollupTransactions, _Mapping]]] = ..., rollup_transactions_proof: _Optional[_Union[Proof, _Mapping]] = ..., rollup_ids_proof: _Optional[_Union[Proof, _Mapping]] = ..., block_hash: _Optional[bytes] = ..., upgrade_change_hashes: _Optional[_Iterable[bytes]] = ..., extended_commit_info_with_proof: _Optional[_Union[ExtendedCommitInfoWithProof, _Mapping]] = ...) -> None: ...

class SequencerBlockHeader(_message.Message):
    __slots__ = ("chain_id", "height", "time", "data_hash", "proposer_address", "rollup_transactions_root")
    CHAIN_ID_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    TIME_FIELD_NUMBER: _ClassVar[int]
    DATA_HASH_FIELD_NUMBER: _ClassVar[int]
    PROPOSER_ADDRESS_FIELD_NUMBER: _ClassVar[int]
    ROLLUP_TRANSACTIONS_ROOT_FIELD_NUMBER: _ClassVar[int]
    chain_id: str
    height: int
    time: _timestamp_pb2.Timestamp
    data_hash: bytes
    proposer_address: bytes
    rollup_transactions_root: bytes
    def __init__(self, chain_id: _Optional[str] = ..., height: _Optional[int] = ..., time: _Optional[_Union[_timestamp_pb2.Timestamp, _Mapping]] = ..., data_hash: _Optional[bytes] = ..., proposer_address: _Optional[bytes] = ..., rollup_transactions_root: _Optional[bytes] = ...) -> None: ...

class GetUpgradesInfoRequest(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class GetUpgradesInfoResponse(_message.Message):
    __slots__ = ("applied", "scheduled")
    class ChangeInfo(_message.Message):
        __slots__ = ("activation_height", "change_name", "app_version", "base64_hash")
        ACTIVATION_HEIGHT_FIELD_NUMBER: _ClassVar[int]
        CHANGE_NAME_FIELD_NUMBER: _ClassVar[int]
        APP_VERSION_FIELD_NUMBER: _ClassVar[int]
        BASE64_HASH_FIELD_NUMBER: _ClassVar[int]
        activation_height: int
        change_name: str
        app_version: int
        base64_hash: str
        def __init__(self, activation_height: _Optional[int] = ..., change_name: _Optional[str] = ..., app_version: _Optional[int] = ..., base64_hash: _Optional[str] = ...) -> None: ...
    APPLIED_FIELD_NUMBER: _ClassVar[int]
    SCHEDULED_FIELD_NUMBER: _ClassVar[int]
    applied: _containers.RepeatedCompositeFieldContainer[GetUpgradesInfoResponse.ChangeInfo]
    scheduled: _containers.RepeatedCompositeFieldContainer[GetUpgradesInfoResponse.ChangeInfo]
    def __init__(self, applied: _Optional[_Iterable[_Union[GetUpgradesInfoResponse.ChangeInfo, _Mapping]]] = ..., scheduled: _Optional[_Iterable[_Union[GetUpgradesInfoResponse.ChangeInfo, _Mapping]]] = ...) -> None: ...
