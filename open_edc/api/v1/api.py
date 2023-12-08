from __future__ import annotations

from open_edc.api.v1.routes import health, login
from open_edc.core.utils import APIRouter

api_router = APIRouter()
api_router.include_router(health.router)
api_router.include_router(login.router)
