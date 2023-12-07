async def test_health(test_client):
    result = await test_client.get("health")

    assert result.status_code == 200
    assert result.json()["system"] == "healthy"
