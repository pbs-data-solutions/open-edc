from open_edc.api.v1.routes import health
from open_edc.core.utils import APIRouter

api_router = APIRouter()
api_router.include_router(health.router)
