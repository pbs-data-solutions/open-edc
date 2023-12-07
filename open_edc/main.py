from contextlib import asynccontextmanager
from typing import AsyncGenerator

import uvicorn
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from open_edc.api.deps import logger
from open_edc.api.v1.api import api_router
from open_edc.db import init_db


@asynccontextmanager  # type: ignore
async def lifespan(app: FastAPI) -> AsyncGenerator:
    logger.info("Initializing the database")
    await init_db()
    yield


app = FastAPI(lifespan=lifespan)
app.include_router(api_router)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


if __name__ == "__main__":
    uvicorn.run(app)
