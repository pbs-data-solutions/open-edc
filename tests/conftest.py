import pytest
from httpx import AsyncClient

from open_edc.core.config import config
from open_edc.main import app


@pytest.fixture
async def test_client():
    async with AsyncClient(app=app, base_url=f"http://127.0.0.1{config.V1_API_PREFIX}") as client:
        yield client
