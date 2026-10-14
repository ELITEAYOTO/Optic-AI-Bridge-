"""Explicit volatile expected-identity comparison. Not trusted enrollment."""
from dataclasses import dataclass

from create_only_successor import SuccessorError
from relative_public_pair import RelativePublicPair


@dataclass(frozen=True)
class PublicRootIdentity:
    volume: int
    file_index: int


def _identity(prepared):
    if prepared.root is None or prepared.close_uncertain:
        raise SuccessorError('root_identity.closed')
    info = prepared._info(prepared.root, True)
    return PublicRootIdentity(info.volume, (info.index_high << 32) | info.index_low)


def capture_public_root(pair):
    # Only an active validated owned fixture reference, not a supplied handle.
    if not isinstance(pair, RelativePublicPair) or pair.operation is None or len(pair.handles) != 3:
        raise SuccessorError('root_identity.unopened')
    return _identity(pair.prepared)


class ExpectedRootPublicPair(RelativePublicPair):
    def __init__(self, directory, expected):
        if (type(expected) is not PublicRootIdentity or type(expected.volume) is not int
                or type(expected.file_index) is not int or not 0 <= expected.volume <= 0xffffffff
                or not 0 < expected.file_index <= 0xffffffffffffffff):
            raise SuccessorError('root_identity.invalid_expected')
        self.expected = expected
        super().__init__(directory)

    def _validate_root(self):
        if _identity(self.prepared) != self.expected:
            raise SuccessorError('root_identity.changed')
