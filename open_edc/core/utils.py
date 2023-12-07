from __future__ import annotations

from typing import Any, Callable

from bson import ObjectId
from bson.errors import InvalidId
from fastapi import APIRouter as FastAPIRouter
from fastapi.types import DecoratedCallable

from open_edc.api.deps import logger


class APIRouter(FastAPIRouter):
    def api_route(
        self, path: str, *, include_in_schema: bool = True, **kwargs: Any
    ) -> Callable[[DecoratedCallable], DecoratedCallable]:
        """Updated api_route function that automatically configures routes to have 2 versions.

        One without a trailing slash and another with it.
        """
        if path.endswith("/"):
            path = path[:-1]

        add_path = super().api_route(path, include_in_schema=include_in_schema, **kwargs)

        alternate_path = f"{path}/"
        add_alternate_path = super().api_route(alternate_path, include_in_schema=False, **kwargs)

        def decorator(func: DecoratedCallable) -> DecoratedCallable:
            add_alternate_path(func)
            return add_path(func)

        return decorator


def str_to_oid(id_str: str) -> ObjectId:
    try:
        return ObjectId(id_str)
    except InvalidId:
        logger.error("%s is not a valid ObjectId", id_str)
        raise
