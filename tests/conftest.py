import pytest
from httpx import AsyncClient
from pymongo.errors import OperationFailure

from open_edc.core.config import config
from open_edc.db import init_db
from open_edc.main import app
from open_edc.models.user import User


@pytest.fixture
async def initialize_db():
    try:
        await init_db()
    except OperationFailure:  # init_db already ran
        pass


# @pytest.fixture(autouse=True)
async def clear_db():
    yield
    await User.delete_all()


@pytest.fixture
async def test_client():
    async with AsyncClient(app=app, base_url=f"http://127.0.0.1{config.V1_API_PREFIX}") as client:
        yield client
