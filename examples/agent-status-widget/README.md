# Agent status test widget

This is a small, read-only Synara widget backed by an agent-written priority list. It uses Node's built-in HTTP server and needs no database or separate container for a local trial. The widget requests **no Matrix capabilities**; an agent updates the list through a separate, token-protected local API.

## Local trial

From this directory, set a long random write token and start the server:

```sh
export SYNARA_WIDGET_WRITE_TOKEN="$(openssl rand -hex 32)"
node server.mjs
```

In Synara Desktop, open **Settings → General → Widgets**, enable experimental widgets, and add an agent widget with this URL:

```text
http://127.0.0.1:8765/?widgetId=$matrix_widget_id
```

Open a room's Widgets panel, select **Agent status**, and leave **Allow sending m.room.message** off. The generated widget ID lets the Rust SDK driver complete its content-loaded and capability handshake. A normal browser visit to `http://127.0.0.1:8765/` also displays the test list.

An agent on the same machine can replace the full list with:

```sh
curl --fail-with-body -X PUT http://127.0.0.1:8765/api/status \
  -H "Authorization: Bearer $SYNARA_WIDGET_WRITE_TOKEN" \
  -H 'Content-Type: application/json' \
  --data '{"items":[{"id":"release-check","title":"Review release checklist","status":"active","priority":1,"detail":"Desktop smoke pass in progress"},{"id":"widget-demo","title":"Try the status widget","status":"pending","priority":2}]}'
```

The widget polls every five seconds. `status` is `pending`, `active`, `blocked`, or `done`; `priority` is 1–3. `PUT` replaces the snapshot and stores it in `data/status.json` with owner-only file permissions. Treat the token as a credential and keep it out of room messages and widget URLs.

## Later hosting

For a local trial, an LXC container is unnecessary. Synara's agent-widget URL policy accepts loopback HTTP, but rejects private LAN HTTP addresses. If agents on other machines need this widget, host it behind an HTTPS name, add authentication for **both reads and writes**, and place the Node process behind a reverse proxy. The sample server deliberately binds to `127.0.0.1`; it is not a public deployment recipe. An alternative is to have a remote agent write through an authenticated tunnel to the local server.

The next iteration could let agents publish structured status events into a Matrix room and have the widget read them through the SDK's widget capabilities. That would remove the separate status API, but it needs a schema, event authorization, and careful handling of encrypted rooms before implementation.
