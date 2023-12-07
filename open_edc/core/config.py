import logging
from typing import Final

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    V1_API_PREFIX: Final[str] = "/api/v1"

    log_level: int = logging.INFO
    mongo_initdb_database: str = "mongo_test"
    mongo_initdb_root_username: str = "mongo"
    mongo_initdb_root_password: str = "mongo_password"
    mongo_host: str = "mongodb://127.0.0.1"
    mongo_port: int = 27017


config = Settings()  # type: ignore
