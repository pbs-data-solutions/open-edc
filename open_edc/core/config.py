import logging
from typing import Final

from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    V1_API_PREFIX: Final[str] = "/api/v1"

    log_level: int = logging.INFO


config = Settings()  # type: ignore
