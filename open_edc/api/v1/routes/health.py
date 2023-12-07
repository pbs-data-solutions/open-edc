from typing import Dict

from open_edc.core.config import config
from open_edc.core.utils import APIRouter

router = APIRouter(tags=["Health"], prefix=config.V1_API_PREFIX)


@router.get("/health")  # , include_in_schema=False)
async def health() -> Dict[str, str]:
    return {"system": "healthy"}
