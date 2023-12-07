import pytest
from httpx import AsyncClient
from motor.motor_asyncio import AsyncIOMotorClient


@pytest.fixture
async def test_client_bad_db():
    from open_edc.api.deps import get_db_client
    from open_edc.core.config import config
    from open_edc.main import app

    def get_db_client_override():
        return AsyncIOMotorClient(
            host="bad",
            username=config.mongo_initdb_root_username,
            password=config.mongo_initdb_root_password,
            port=config.mongo_port,
            serverSelectionTimeoutMS=100,
        )

    app.dependency_overrides[get_db_client] = get_db_client_override

    async with AsyncClient(app=app, base_url=f"http://127.0.0.1{config.V1_API_PREFIX}") as client:
        yield client

    app.dependency_overrides = {}


async def test_health(test_client):
    result = await test_client.get("health")

    assert result.status_code == 200
    assert result.json()["system"] == "healthy"
    assert result.json()["db"] == "healthy"


async def test_health_unhealthy_db(test_client_bad_db):
    result = await test_client_bad_db.get("health")

    assert result.status_code == 200
    assert result.json()["db"] == "unhealthy"
