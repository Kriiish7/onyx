"""
Exception hierarchy for the Onyx SDK.
"""


class OnyxError(Exception):
    """Base exception for all Onyx SDK errors."""

    pass


class OnyxApiError(OnyxError):
    """The server returned an HTTP error response.

    Attributes:
        status_code: The HTTP status code.
        message: The error message from the server.
    """

    def __init__(self, status_code: int, message: str) -> None:
        self.status_code = status_code
        self.message = message
        super().__init__(f"API error ({status_code}): {message}")


class OnyxNetworkError(OnyxError):
    """A network / transport error occurred."""

    pass


class OnyxNotFoundError(OnyxApiError):
    """The requested resource was not found (HTTP 404).

    Attributes:
        status_code: Always 404.
        message: Description of what was not found.
    """

    def __init__(self, message: str) -> None:
        super().__init__(404, message)


class OnyxValidationError(OnyxError):
    """An invalid argument was provided to the SDK."""

    pass
