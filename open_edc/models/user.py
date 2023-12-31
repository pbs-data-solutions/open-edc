from __future__ import annotations

from datetime import datetime

from beanie import Document
from camel_converter.pydantic_base import CamelBase
from pydantic import Field
from pymongo import ASCENDING, IndexModel

from open_edc.models.object_id import ObjectIdStr


class _UserBase(CamelBase):
    user_name: str
    first_name: str
    last_name: str


class PasswordReset(CamelBase):
    user_name: str
    security_question_answer: str
    new_password: str


class UserCreate(_UserBase):
    password: str
    security_question_answer: str


class UserNoPassword(_UserBase):
    id: ObjectIdStr
    is_active: bool
    is_admin: bool

    class Settings:
        projection = {
            "id": "$_id",
            "user_name": "$user_name",
            "first_name": "$first_name",
            "last_name": "$last_name",
            "is_active": "$is_active",
            "is_admin": "$is_admin",
        }


class UserUpdateMe(_UserBase):
    id: ObjectIdStr
    password: str
    security_question_answer: str


class UserUpdate(UserUpdateMe):
    is_active: bool
    is_admin: bool


class User(Document):
    user_name: str
    first_name: str
    last_name: str
    hashed_password: str
    security_question_answer: str
    is_active: bool = True
    is_admin: bool = False
    date_created: datetime = Field(default_factory=datetime.now)
    last_update: datetime = Field(default_factory=datetime.now)
    last_login: datetime = Field(default_factory=datetime.now)

    class Settings:
        name = "users"
        indexes = [
            IndexModel(keys=[("user_name", ASCENDING)], name="user_name", unique=True),
            IndexModel(keys=[("is_active", ASCENDING)], name="is_active"),
            IndexModel(keys=[("is_admin", ASCENDING)], name="is_admin"),
        ]
