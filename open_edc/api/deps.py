from __future__ import annotations

import logging
from typing import Annotated

from fastapi import Depends
from motor.motor_asyncio import AsyncIOMotorClient

from open_edc.core.config import config
from open_edc.db import db_client

logging.basicConfig(format="%asctime)s - %(levelname)s - [%(filename)s:%(lineno)d] - %(message)s")
logging.root.setLevel(level=config.log_level)
logger = logging


# motor 3.3.0 broke types see: https://www.mongodb.com/community/forums/t/motor-3-3-0-released/241116
# and https://jira.mongodb.org/browse/MOTOR-1177
def get_db_client() -> AsyncIOMotorClient:  # type: ignore
    return db_client


MongoClient = Annotated[AsyncIOMotorClient, Depends(get_db_client)]  # type: ignore
