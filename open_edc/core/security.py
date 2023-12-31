from __future__ import annotations

from datetime import datetime, timedelta, timezone
from typing import Any

import jwt
from beanie import PydanticObjectId
from passlib.hash import argon2

from open_edc.core.config import config

ALGORITHM = "HS256"
DEFAULT_ROUNDS = 25


def create_access_token(
    subject: PydanticObjectId | str | dict[str, Any], expires_delta: timedelta | None = None
) -> str:
    if expires_delta:
        expire = datetime.now(tz=timezone.utc) + expires_delta
    else:
        expire = datetime.now(tz=timezone.utc) + timedelta(
            minutes=config.ACCESS_TOKEN_EXPIRE_MINUTES
        )
    to_encode = {"exp": expire, "sub": str(subject)}
    encoded_jwt = jwt.encode(to_encode, config.SECRET_KEY, algorithm=ALGORITHM)
    return encoded_jwt


def verify_password(plain_password: str, hashed_password: str) -> bool:
    return argon2.verify(plain_password, hashed_password)


def get_password_hash(password: str, *, _rounds: int = DEFAULT_ROUNDS) -> str:
    return argon2.using(rounds=_rounds).hash(password)
