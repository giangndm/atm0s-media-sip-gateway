# SIP Gateway for atm0s-media-server

## Overview

This project implements a SIP gateway in Rust, using Tokio for asynchronous operations. It's designed to manage SIP calls, handle media streams, and synchronize with an address book.  Key features include incoming/outgoing call management, media handling via a media server, address book integration, secure tokens, WebSocket support, and incoming call handling via WebSockets.

## Inputs

### Create Call Request

To initiate a call, the following information is required:

*   **sip_server:** The address of the SIP server.
*   **sip_auth (optional):** SIP authentication credentials (username, password).
*   **from_number:** The originating phone number.
*   **to_number:** The destination phone number.
*   **hook:** A URL for receiving call event notifications (webhooks).
*   **streaming:** Information for media streaming (room, peer, record).

### Incoming Call

Incoming calls trigger events that require specific actions. The gateway expects responses to these events to manage the call flow.  These actions include `Ring`, `Accept`, and `End`.

*   **Ring:** Signals the call is ringing.
*   **Accept:** Accepts the incoming call and provides streaming information.
*   **End:** Terminates the call.

## Outputs

### Create Call Response

A successful create call request returns:

*   **call_id:** A unique identifier for the call.
*   **call_token:** A secure token for WebSocket communication.
*   **call_ws:** The WebSocket URL for interacting with the call.

### Call Events

Throughout the call lifecycle, various events are emitted via WebSockets and webhooks.  These events provide updates on the call status. Examples include:

*   **Outgoing Call Events:** `Provisional`, `Early`, `Accepted`, `Failure`, `Bye`, `Ended`, `Error`.
*   **Incoming Call Events:** `Cancelled`, `Bye`, `Accepted`, `Ended`, `Error`.
*   **Incoming Call Notifications:** `CallArrived`, `CallCancelled`, `CallAccepted`.

### Errors

Errors are returned as JSON objects with a `status` field set to `false` and an `error` field containing a description of the error.  Possible errors include:

*   **BadRequest:** Indicates an invalid request format.
*   **WrongSecret:**  An incorrect application secret was provided.
*   **WrongToken:** An invalid or expired call token was used.
*   **SipError:** An error occurred within the SIP stack.
*   **InternalChannel:** An internal communication error occurred.
