# Traffic Replay Design

## Goal

Add a "replay request" action to the traffic session context menu. The action re-sends a captured request directly from the desktop backend and records the replay result as a new traffic session.

## User Flow

On the monitor page, the user right-clicks a traffic row and chooses "重放请求". The app closes the context menu, sends the selected session id to the backend, and shows success or failure feedback. A successful replay appears as a new row in the existing traffic list through the current `traffic://batch` event path.

## Backend Behavior

Add a Tauri command named `traffic_replay(id)`.

The command reads the original `TransactionDetail` from `SharedTraffic`. If the detail is missing, it returns a clear error because the cached session has expired or was deleted.

The command reuses the original request method, URL, headers, and captured request body. It sends the request directly from the backend instead of routing through Flowly's proxy listener. This avoids requiring the proxy to be running and prevents replay loops caused by the proxy capturing its own replay request.

The replay result is recorded with a new traffic id. It uses the same `SharedTraffic::begin_request` and `SharedTraffic::complete` path as proxied traffic, so the frontend receives it without a new event channel.

## Header Handling

Replay skips hop-by-hop or connection-scoped headers that should not be copied to a new client request:

- `connection`
- `keep-alive`
- `proxy-authenticate`
- `proxy-authorization`
- `te`
- `trailer`
- `transfer-encoding`
- `upgrade`
- `host`
- `content-length`

The HTTP client computes `Host` and `Content-Length` from the target URL and body.

## Body Handling

If the original request body was captured, replay sends that body. If the original body was not captured, replay sends an empty body. This keeps the first implementation deterministic and avoids guessing bytes that are no longer available.

## Frontend Behavior

Add `replay(id)` to the traffic Pinia store. Add a "重放请求" item to the existing monitor context menu. The action uses the row that opened the context menu, not the whole current selection, because replay is a single-session action.

On success, show "已重放请求". On failure, show "重放失败：<error>".

## Testing

Add a backend integration test that:

1. Starts the existing test HTTP server.
2. Captures an initial POST request through the proxy.
3. Calls the replay function or command path for the captured session.
4. Verifies the test server received the replayed request.
5. Verifies `SharedTraffic` now contains a second session for the replay result.

Frontend behavior is simple command wiring and can be verified by build unless the project later adds a frontend test harness.
