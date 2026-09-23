const widgetId = new URL(location.href).searchParams.get("widgetId");

// The Rust SDK's widget driver waits for content_loaded, then asks what
// capabilities this widget wants. This sample uses its own local status API,
// so it requests no Matrix room or send capabilities.
if (widgetId) {
  window.addEventListener("message", (event) => {
    if (event.source !== window || event.origin !== location.origin) return;
    const message = event.data;
    if (!message || message.api !== "toWidget" || "response" in message) return;
    if (message.action === "capabilities") {
      window.postMessage(
        { ...message, response: { capabilities: [] } },
        location.origin
      );
    } else if (message.action === "notify_capabilities") {
      window.postMessage({ ...message, response: {} }, location.origin);
    }
  });
  window.postMessage(
    {
      api: "fromWidget",
      widgetId,
      requestId: crypto.randomUUID(),
      action: "content_loaded",
      data: {},
    },
    location.origin
  );
}

const counts = ["active", "blocked", "pending"];
const itemsNode = document.getElementById("items");
const errorNode = document.getElementById("error");

async function refresh() {
  try {
    const response = await fetch("/api/status", { cache: "no-store" });
    if (!response.ok)
      throw new Error(`Status request failed (${response.status}).`);
    const snapshot = await response.json();
    const items = Array.isArray(snapshot.items) ? snapshot.items : [];
    items.sort(
      (a, b) => a.priority - b.priority || a.title.localeCompare(b.title)
    );
    for (const status of counts) {
      document.getElementById(`${status}-count`).textContent = items.filter(
        (item) => item.status === status
      ).length;
    }
    document.getElementById("updated").textContent = snapshot.updatedAt
      ? `Updated ${new Date(snapshot.updatedAt).toLocaleString()}`
      : "Sample data — waiting for the first agent update";
    itemsNode.replaceChildren();
    for (const item of items) {
      const row = document.createElement("li");
      row.className = `item ${item.status}`;
      const title = document.createElement("strong");
      title.textContent = item.title;
      const detail = document.createElement("p");
      detail.textContent = item.detail || "";
      const meta = document.createElement("small");
      meta.textContent = `P${item.priority} · ${item.status}`;
      row.append(title, meta, detail);
      itemsNode.append(row);
    }
    errorNode.textContent = "";
  } catch (error) {
    errorNode.textContent = error.message || "Could not load status.";
  }
}

void refresh();
setInterval(refresh, 5000);
