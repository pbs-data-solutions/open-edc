from uuid import uuid4

import pytest
from bson import ObjectId


async def test_create_user(test_client):
    user_data = {
        "userName": str(uuid4),
        "firstName": "Imma",
        "lastName": "User",
        "password": "immapassword",
        "securityQuestionAnswer": "my answer",
    }
    response = await test_client.post("user/", json=user_data)
    response_json = response.json()

    assert response_json["userName"] == user_data["userName"]


async def test_create_user_duplicate(mock_user, test_client):
    user_data = {
        "userName": mock_user.user_name,
        "firstName": "Imma",
        "lastName": "User",
        "password": "immapassword",
        "securityQuestionAnswer": "my answer",
    }
    response = await test_client.post("user/", json=user_data)

    assert response.status_code == 400


async def test_get_users(mock_user, test_client, superuser_token_headers):
    response = await test_client.get("user/", headers=superuser_token_headers)
    response_json = response.json()

    assert len(response_json) == 2
    assert response_json[0]["userName"] == mock_user.user_name


@pytest.mark.usefixtures("mock_user")
async def test_get_users_not_admin(test_client, user_token_headers):
    response = await test_client.get("user/", headers=user_token_headers)

    assert response.status_code == 403
    assert "required permissions" in response.json()["detail"]


@pytest.mark.usefixtures("mock_user")
async def test_get_users_not_authenticated(test_client):
    response = await test_client.get("user/")

    assert response.status_code == 401


async def test_get_user_by_id(mock_user, test_client, superuser_token_headers):
    response = await test_client.get(f"user/{mock_user.id}", headers=superuser_token_headers)
    response_json = response.json()

    assert response_json["userName"] == mock_user.user_name


async def test_get_user_by_id_bad_id(test_client, superuser_token_headers):
    response = await test_client.get("user/bad", headers=superuser_token_headers)

    assert response.status_code == 400
    assert "not a valid ID format" in response.json()["detail"]


async def test_get_user_by_id_not_admin(mock_user, test_client, user_token_headers):
    response = await test_client.get(f"user/{mock_user.id}", headers=user_token_headers)

    assert response.status_code == 403
    assert "required permissions" in response.json()["detail"]


async def test_get_user_by_id_not_authenticated(mock_user, test_client):
    response = await test_client.get(f"user/{mock_user.id}")

    assert response.status_code == 401


async def test_get_user_by_name(mock_user, test_client, superuser_token_headers):
    response = await test_client.get(
        f"user/user-name/{mock_user.user_name}", headers=superuser_token_headers
    )
    response_json = response.json()

    assert response_json["userName"] == mock_user.user_name


async def test_get_user_by_name_not_found(test_client, superuser_token_headers):
    response = await test_client.get(f"user/user-name/{uuid4()}", headers=superuser_token_headers)

    assert response.status_code == 404


@pytest.mark.usefixtures("mock_user")
async def test_get_user_by_name_not_admin(test_client, user_token_headers):
    response = await test_client.get(f"user/user-name/{uuid4()}", headers=user_token_headers)

    assert response.status_code == 403
    assert "required permissions" in response.json()["detail"]


async def test_get_user_by_name_not_authenticated(test_client):
    response = await test_client.get(f"user/user-name/{uuid4()}")

    assert response.status_code == 401


async def test_get_me(test_client, mock_user, user_token_headers):
    response = await test_client.get("user/me", headers=user_token_headers)

    assert response.json()["id"] == str(mock_user.id)


async def test_get_me_not_authenticated(test_client):
    response = await test_client.get("user/me")

    assert response.status_code == 401


async def test_delete_user_by_id(mock_user, test_client, superuser_token_headers):
    response = await test_client.delete(f"user/{mock_user.id}", headers=superuser_token_headers)
    assert response.status_code == 204

    response = await test_client.get(f"user/{mock_user.id}", headers=superuser_token_headers)
    assert response.status_code == 404


async def test_delete_user_by_id_bad_id(test_client, superuser_token_headers):
    response = await test_client.delete("user/bad", headers=superuser_token_headers)
    assert response.status_code == 400
    assert "not a valid ID format" in response.json()["detail"]


async def test_delete_user_by_id_not_found(test_client, superuser_token_headers):
    response = await test_client.delete(f"user/{str(ObjectId())}", headers=superuser_token_headers)

    assert response.status_code == 404


async def test_delete_user_by_id_not_admin(test_client, mock_user, user_token_headers):
    response = await test_client.delete(f"user/{mock_user.id}", headers=user_token_headers)

    assert response.status_code == 403
    assert "required permissions" in response.json()["detail"]


async def test_delete_user_by_id_not_authenticated(test_client, mock_user):
    response = await test_client.delete(f"user/{mock_user.id}")

    assert response.status_code == 401


async def test_delete_user_by_userName(mock_user, test_client, superuser_token_headers):
    response = await test_client.delete(
        f"user/user-name/{mock_user.user_name}", headers=superuser_token_headers
    )
    assert response.status_code == 204

    response = await test_client.get(f"user/{mock_user.id}", headers=superuser_token_headers)
    assert response.status_code == 404


async def test_delete_user_by_userName_not_found(test_client, superuser_token_headers):
    response = await test_client.delete(
        f"user/user-name/{uuid4()}", headers=superuser_token_headers
    )

    assert response.status_code == 404


async def test_delete_user_by_userName_not_admin(mock_user, test_client, user_token_headers):
    response = await test_client.delete(
        f"user/user-name/{mock_user.user_name}", headers=user_token_headers
    )
    assert response.status_code == 403
    assert "required permissions" in response.json()["detail"]


async def test_delete_user_by_userName_not_authenticated(mock_user, test_client):
    response = await test_client.delete(f"user/user-name/{mock_user.user_name}")
    assert response.status_code == 401


async def test_delete_me(mock_user, test_client, user_token_headers, superuser_token_headers):
    response = await test_client.delete("user/me", headers=user_token_headers)
    assert response.status_code == 204

    response = await test_client.get(f"user/{mock_user.id}", headers=superuser_token_headers)
    assert response.status_code == 404


async def test_delete_me_not_authenticated(test_client):
    response = await test_client.delete("user/me")
    assert response.status_code == 401


async def test_update_me(mock_user, user_data, test_client, user_token_headers):
    user_data.pop("_id")
    user_data["password"] = "newPassword"
    user_data["userName"] = str(uuid4())
    user_data["id"] = str(mock_user.id)
    response = await test_client.put("user/me", headers=user_token_headers, json=user_data)

    assert response.json()["userName"] == user_data["userName"]


@pytest.mark.usefixtures("mock_user")
async def test_update_me_different_user(user_data, test_client, user_token_headers):
    response = await test_client.post(
        "user",
        json={
            "userName": str(uuid4()),
            "firstName": "Imma",
            "lastName": "User",
            "password": "abc",
            "securityQuestionAnswer": "my answer",
        },
    )
    assert response.status_code == 200
    user_data.pop("_id")
    user_data["password"] = "newPassword"
    user_data["userName"] = str(uuid4())
    user_data["id"] = response.json()["id"]
    response = await test_client.put("user/me", headers=user_token_headers, json=user_data)

    assert response.status_code == 400
    assert "Invalid user ID" == response.json()["detail"]


async def test_update_me_duplicate_userName(test_client, mock_user, user_token_headers):
    user_data = {
        "userName": str(uuid4()),
        "firstName": "Imma",
        "lastName": "User",
        "password": "some_password",
        "securityQuestionAnswer": "my answer",
    }
    response = await test_client.post("user/", json=user_data)
    assert response.status_code == 200
    user_data["id"] = str(mock_user.id)
    response = await test_client.put("user/me", headers=user_token_headers, json=user_data)

    assert response.status_code == 400


async def test_update_me_not_authenticated(mock_user, user_data, test_client):
    user_data.pop("_id")
    user_data["password"] = "newPassword"
    user_data["userName"] = str(uuid4())
    user_data["id"] = str(mock_user.id)
    response = await test_client.put("user/me", json=user_data)

    assert response.status_code == 401


@pytest.mark.usefixtures("mock_user")
async def test_forgot_password(user_data, test_client):
    reset_info = {
        "userName": user_data["user_name"],
        "securityQuestionAnswer": user_data["security_question_answer"],
        "newPassword": "new",
    }

    response = await test_client.patch("user/forgot-password", json=reset_info)

    assert response.status_code == 200


@pytest.mark.usefixtures("mock_user")
async def test_forgot_password_wrong_answer(user_data, test_client):
    reset_info = {
        "userName": user_data["user_name"],
        "securityQuestionAnswer": "bad",
        "newPassword": "new",
    }

    response = await test_client.patch("user/forgot-password", json=reset_info)

    assert response.status_code == 400


@pytest.mark.usefixtures("mock_user")
async def test_forgot_password_not_found(user_data, test_client):
    reset_info = {
        "userName": "unknown",
        "securityQuestionAnswer": user_data["security_question_answer"],
        "newPassword": "new",
    }

    response = await test_client.patch("user/forgot-password", json=reset_info)

    assert response.status_code == 404
