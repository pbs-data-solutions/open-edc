from __future__ import annotations

import logging
from typing import Annotated

import jwt
from bson import ObjectId
from bson.errors import InvalidId
from fastapi import Depends, HTTPException
from fastapi.security import OAuth2PasswordBearer
from jwt.exceptions import PyJWTError
from motor.motor_asyncio import AsyncIOMotorClient
from pydantic import ValidationError
from starlette.status import HTTP_403_FORBIDDEN, HTTP_404_NOT_FOUND

from open_edc.core.config import Settings, config
from open_edc.core.security import ALGORITHM
from open_edc.db import db_client
from open_edc.models.token import TokenPayload
from open_edc.models.user import UserNoPassword
from open_edc.services.user_service import get_user

logging.basicConfig(format="%asctime)s - %(levelname)s - [%(filename)s:%(lineno)d] - %(message)s")
logging.root.setLevel(level=config.log_level)
logger = logging

_oauth2_scheme = OAuth2PasswordBearer(tokenUrl=f"{config.V1_API_PREFIX}/login/access-token")


def get_config() -> Settings:
    return config


async def get_current_user(token: Annotated[str, Depends(_oauth2_scheme)]) -> UserNoPassword:
    try:
        payload = jwt.decode(token, config.SECRET_KEY, algorithms=[ALGORITHM])
        token_data = TokenPayload(**payload)
    except (PyJWTError, ValidationError) as e:
        logger.info("Could not validate credentials: %s", e)
        raise HTTPException(status_code=HTTP_403_FORBIDDEN, detail="Could not validate credentials")

    try:
        oid = ObjectId(token_data.sub)
    except InvalidId:  # pragma: no cover
        logger.info("%s is not a valid ObjectId", token_data.sub)
        raise HTTPException(
            status_code=HTTP_403_FORBIDDEN, detail=f"{token_data.sub} is not a valid ID format"
        )

    user = await get_user(oid)

    if not user:
        logger.info("User not found")
        raise HTTPException(status_code=HTTP_404_NOT_FOUND, detail="User not found")

    return user


# motor 3.3.0 broke types see: https://www.mongodb.com/community/forums/t/motor-3-3-0-released/241116
# and https://jira.mongodb.org/browse/MOTOR-1177
def get_db_client() -> AsyncIOMotorClient:  # type: ignore
    return db_client


Config = Annotated[Settings, Depends(get_config)]
CurrentUser = Annotated[UserNoPassword, Depends(get_current_user)]
MongoClient = Annotated[AsyncIOMotorClient, Depends(get_db_client)]  # type: ignore
