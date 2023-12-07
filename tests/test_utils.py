import pytest
from bson import ObjectId
from bson.errors import InvalidId

from open_edc.core.utils import str_to_oid


def test_str_to_oid():
    oid = ObjectId()
    result = str_to_oid(str(oid))
    assert oid == result


def test_str_to_oid_bad_id():
    with pytest.raises(InvalidId):
        str_to_oid("bad")
