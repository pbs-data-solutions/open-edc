from __future__ import annotations


class DuplicateUserNameError(Exception):
    pass


class InvalidApiKeyError(Exception):
    pass


class NoRecordsDeletedError(Exception):
    pass


class NoRecordsUpdatedError(Exception):
    pass


class SecurityQuestionMismatch(Exception):
    pass


class UserNotFoundError(Exception):
    pass
