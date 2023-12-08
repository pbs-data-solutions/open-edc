import asyncio
from uuid import uuid4

import pytest
from bson import ObjectId
from httpx import AsyncClient
from pymongo.errors import OperationFailure

from open_edc.core.config import config
from open_edc.core.security import get_password_hash
from open_edc.db import init_db
from open_edc.main import app
from open_edc.models.user import User


@pytest.fixture(scope="session", autouse=True)
def event_loop():
    try:
        loop = asyncio.get_running_loop()
    except RuntimeError:
        loop = asyncio.new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(scope="session", autouse=True)
async def initialize_db():
    try:
        await init_db()
    except OperationFailure:  # init_db already ran
        pass


@pytest.fixture(autouse=True)
async def clear_db():
    models = (User,)
    yield
    for model in models:
        try:
            await model.delete_all()
        except Exception:
            pass


@pytest.fixture(scope="session")
async def test_client():
    async with AsyncClient(app=app, base_url=f"http://127.0.0.1{config.V1_API_PREFIX}") as client:
        yield client


@pytest.fixture
def user_data():
    return {
        "_id": str(ObjectId()),
        "user_name": str(uuid4()),
        "first_name": "Imma",
        "last_name": "User",
        "hashed_password": get_password_hash("test_password", _rounds=1),
        "security_question_answer": "my answer",
    }


@pytest.fixture
async def mock_user(user_data):
    return await User(**user_data).insert()


@pytest.fixture
async def admin_user():
    return await User(
        user_name=str(uuid4()),
        first_name="Admin",
        last_name="User",
        hashed_password=get_password_hash("test_password", _rounds=1),
        security_question_answer="my answer",
        is_admin=True,
    ).insert()


@pytest.fixture
async def superuser_token_headers(test_client, admin_user):
    login_data = {
        "username": admin_user.user_name,
        "first_name": admin_user.first_name,
        "last_name": admin_user.last_name,
        "password": "test_password",
    }
    response = await test_client.post("/login/access-token", data=login_data)
    tokens = response.json()
    a_token = tokens["accessToken"]
    headers = {"Authorization": f"Bearer {a_token}"}
    return headers


@pytest.fixture
async def user_token_headers(test_client, user_data):
    login_data = {
        "username": user_data["user_name"],
        "password": "test_password",
    }
    response = await test_client.post("/login/access-token", data=login_data)
    tokens = response.json()
    a_token = tokens["accessToken"]
    headers = {"Authorization": f"Bearer {a_token}"}
    return headers
