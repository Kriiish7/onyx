/**
 * Error classes for the Onyx SDK.
 *
 * All SDK errors extend the base {@link OnyxError} class so you can catch
 * `OnyxError` to handle any SDK error, or catch specific subclasses for more
 * fine-grained handling.
 *
 * @module
 */

/**
 * Base class for all Onyx SDK errors.
 */
export class OnyxError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "OnyxError";
  }
}

/**
 * The server returned an HTTP error response.
 *
 * @example
 * ```typescript
 * try {
 *   await client.nodes.get("non-existent-id");
 * } catch (err) {
 *   if (err instanceof OnyxApiError) {
 *     console.error(`HTTP ${err.statusCode}: ${err.message}`);
 *   }
 * }
 * ```
 */
export class OnyxApiError extends OnyxError {
  /** HTTP status code returned by the server. */
  public readonly statusCode: number;

  constructor(statusCode: number, message: string) {
    super(`API error (${statusCode}): ${message}`);
    this.name = "OnyxApiError";
    this.statusCode = statusCode;
  }
}

/**
 * The requested resource was not found (HTTP 404).
 */
export class OnyxNotFoundError extends OnyxApiError {
  constructor(message: string) {
    super(404, message);
    this.name = "OnyxNotFoundError";
  }
}

/**
 * A network or transport error occurred (e.g. connection refused).
 */
export class OnyxNetworkError extends OnyxError {
  constructor(message: string) {
    super(`Network error: ${message}`);
    this.name = "OnyxNetworkError";
  }
}

/**
 * An invalid argument was provided to the SDK.
 */
export class OnyxValidationError extends OnyxError {
  constructor(message: string) {
    super(`Validation error: ${message}`);
    this.name = "OnyxValidationError";
  }
}
