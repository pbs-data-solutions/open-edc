from __future__ import annotations

from beanie.exceptions import RevisionIdWasChanged
from bson import ObjectId
from bson.errors import InvalidId
from fastapi import HTTPException
from pymongo.errors import DuplicateKeyError
from starlette.status import (
    HTTP_204_NO_CONTENT,
    HTTP_400_BAD_REQUEST,
    HTTP_404_NOT_FOUND,
    HTTP_500_INTERNAL_SERVER_ERROR,
)

from open_edc.api.deps import CurrentAdminUser, CurrentUser, logger
from open_edc.core.config import config
from open_edc.core.utils import APIRouter, str_to_oid
from open_edc.exceptions import (
    DuplicateUserNameError,
    NoRecordsDeletedError,
    NoRecordsUpdatedError,
    SecurityQuestionMismatch,
    UserNotFoundError,
)
from open_edc.models.user import PasswordReset, UserCreate, UserNoPassword, UserUpdateMe
from open_edc.services.user_service import create_user as create_user_service
from open_edc.services.user_service import delete_user_by_id as delete_user_by_id_service
from open_edc.services.user_service import (
    delete_user_by_user_name as delete_user_by_user_name_service,
)
from open_edc.services.user_service import forgot_password as forgot_password_service
from open_edc.services.user_service import get_user_by_id as get_user_by_id_service
from open_edc.services.user_service import get_user_by_user_name as get_user_by_user_name_service
from open_edc.services.user_service import get_users as get_user_service
from open_edc.services.user_service import update_me as update_me_service

router = APIRouter(tags=["User"], prefix=f"{config.V1_API_PREFIX}/user")


@router.post("/")
async def create_user(user: UserCreate) -> UserNoPassword:
    """Create a new user."""
    logger.info("Creating user")
    try:
        created_user = await create_user_service(user)
    except DuplicateUserNameError:
        logger.info("A user with the user name %s already exists", user.user_name)
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST,
            detail=f"A user with the user name {user.user_name} already exists",
        )
    except Exception as e:  # pragma: no cover
        logger.error("An error occurred while inserting user: %s", e)
        raise HTTPException(
            status_code=HTTP_500_INTERNAL_SERVER_ERROR,
            detail="An error occurred while creating the user",
        )

    return created_user


@router.patch("/forgot-password")
async def forgot_password(reset_info: PasswordReset) -> UserNoPassword:
    """Reset a forgotten password."""
    logger.info("Resetting the password")

    try:
        update_result = await forgot_password_service(reset_info)
    except UserNotFoundError:
        logger.info(
            "Error resetting the password for user %s: user was not found", reset_info.user_name
        )
        raise HTTPException(
            status_code=HTTP_404_NOT_FOUND,
            detail=f"No user with the user name {reset_info.user_name} found",
        )
    except SecurityQuestionMismatch:
        logger.info(
            "Error resetting the password for user %s: the security question answer does not match",
            reset_info.user_name,
        )
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST,
            detail="The security question answer does not match",
        )
    except NoRecordsUpdatedError:  # pragma: no cover
        logger.info("Error resetting the password for user %s", reset_info.user_name)
        raise HTTPException(
            status_code=HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error resetting the password for user {reset_info.user_name}",
        )
    except Exception as e:  # pragma: no cover
        logger.error(
            "An error occurred while resetting the password for user %s: %s",
            reset_info.user_name,
            e,
        )
        raise HTTPException(
            status_code=HTTP_500_INTERNAL_SERVER_ERROR,
            detail="An error occurred while updating user",
        )

    return update_result


@router.get("/")
async def get_users(_: CurrentAdminUser) -> list[UserNoPassword]:
    """Get all users."""
    logger.info("Getting all users")
    return await get_user_service()


@router.get("/me")
async def get_me(create_user: CurrentUser) -> UserNoPassword:
    """Retriever the logged in user's information."""
    logger.info("Getting current user")
    return UserNoPassword(**create_user.model_dump())


@router.get("/{user_id}")
async def get_user_by_id(user_id: str, _: CurrentAdminUser) -> UserNoPassword:
    """Get a user by ID."""
    logger.info("Getting user %s", user_id)
    try:
        oid = str_to_oid(user_id)
    except InvalidId:
        logger.info("%s is not a valid ID format", user_id)
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST, detail=f"{user_id} is not a valid ID format"
        )

    user = await get_user_by_id_service(oid)

    if not user:
        logger.info("User with id %s not found", user_id)
        raise HTTPException(
            status_code=HTTP_404_NOT_FOUND, detail=f"User with id {user_id} not found"
        )

    return user


@router.get("/user-name/{user_name}")
async def get_user_by_user_name(user_name: str, _: CurrentAdminUser) -> UserNoPassword:
    """Get a user by user name."""
    logger.info("Getting user %s", user_name)
    user = await get_user_by_user_name_service(user_name)

    if not user:
        logger.info("User with user name %s not found", user_name)
        raise HTTPException(
            status_code=HTTP_404_NOT_FOUND, detail=f"User with user name {user_name} not found"
        )

    return user


@router.delete("/me", response_model=None, status_code=HTTP_204_NO_CONTENT)
async def delete_me(current_user: CurrentUser) -> None:
    """Delete the current logged in user."""
    logger.info("Deleting the current user")

    # fail-safe, shouldn't be possible to hit
    if not current_user.id:  # pragma: no cover
        logger.info("User has no id, unable to delete")
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST, detail="User has no ID, unable to delete"
        )

    try:
        await delete_user_by_id_service(ObjectId(current_user.id))
    except NoRecordsDeletedError:  # pragma: no cover
        # fail-safe, shouldn't be possible to hit
        logger.info("User with id %s not found. No delete performed", current_user.id)
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST,
            detail=f"User with id {current_user.id} not found. No user deleted",
        )
    except Exception as e:  # pragma: no cover
        logger.error(
            "An error occurred while deleting user %s: %s",
            current_user.id,
            e,
        )
        raise HTTPException(
            status_code=HTTP_500_INTERNAL_SERVER_ERROR,
            detail="An error occurred while deleting user",
        )


@router.delete("/{user_id}", response_model=None, status_code=HTTP_204_NO_CONTENT)
async def delete_user_by_id(user_id: str, _: CurrentAdminUser) -> None:
    """Delete a user by ID."""
    try:
        oid = ObjectId(user_id)
    except InvalidId:
        logger.info("%s is not a valid ObjectId", user_id)
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST, detail=f"{user_id} is not a valid ID format"
        )

    try:
        await delete_user_by_id_service(oid)
    except NoRecordsDeletedError:
        logger.info("User with id %s not found. No delete performed", user_id)
        raise HTTPException(
            status_code=HTTP_404_NOT_FOUND,
            detail=f"User with id {user_id} not found. No user deleted",
        )


@router.delete("/user-name/{user_name}", response_model=None, status_code=HTTP_204_NO_CONTENT)
async def delete_user_by_user_name(user_name: str, _: CurrentAdminUser) -> None:
    """Delete a user by user name."""
    try:
        await delete_user_by_user_name_service(user_name)
    except NoRecordsDeletedError:
        logger.info("User with user name %s not found. No delete performed", user_name)
        raise HTTPException(
            status_code=HTTP_404_NOT_FOUND,
            detail=f"User with user name {user_name} not found. No user deleted",
        )


@router.put("/me")
async def update_me(user: UserUpdateMe, current_user: CurrentUser) -> UserNoPassword:
    """Update the logged in user's information."""
    logger.info("Updating user")

    if user.id != current_user.id:
        logger.info("Cannot update another user's information")
        raise HTTPException(status_code=HTTP_400_BAD_REQUEST, detail="Invalid user ID")

    try:
        update_result = await update_me_service(user)
    except NoRecordsUpdatedError:  # pragma: no cover
        # Shouldn't be able to get here because we have already checked that the user exists
        logger.info("Error updating user %s", user.id)
        raise HTTPException(
            status_code=HTTP_500_INTERNAL_SERVER_ERROR, detail=f"Error updating user {user.id}"
        )
    except DuplicateKeyError as e:
        logger.info("User name %s already exists: %s", user.user_name, e)
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST, detail="User name {user.user_name} already exists"
        )
    except RevisionIdWasChanged as e:  # pragma: no cover
        # Same as DuplicateKeyError. I'm not sure why sometimes it is one and sometimes it is the
        # other
        logger.info("User name %s already exists: %s", user.user_name, e)
        raise HTTPException(
            status_code=HTTP_400_BAD_REQUEST, detail="User name {user.user_name} already exists"
        )
    except Exception as e:  # pragma: no cover
        logger.info("An error occurred while updating user %s: %s", user.id, e)
        raise HTTPException(
            status_code=HTTP_500_INTERNAL_SERVER_ERROR,
            detail="An error occurred while updating user",
        )

    return update_result
