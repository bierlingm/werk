const PORT_RANGE_START = 3749;
const PORT_RANGE_END = 3759;
const PROBE_TIMEOUT_MS = 800;
const MAX_NEXT = 5;
const MAX_HELD = 5;

let API = null;
let lastGoodPort = null;

const $ = (id) => document.getElementById(id);

const bandOverdue = $("band-overdue");
const bandNext = $("band-next");
const bandHeld = $("band-held");
const bandSilent = $("band-silent");
const bandOffline = $("band-offline");

const listOverdue = $("list-overdue");
const listNext = $("list-next");
const listHeld = $("list-held");

const summaryEl = $("summary");
const statusEl = $("status");

const wsButton = $("workspace-button");
const wsName = $("workspace-name");
const wsMenu = $("workspace-menu");

let currentWorkspace = null;
let workspaceList = [];
let switching = false;

function fmtHorizon(h) {
  return h ? `· ${h}` : "";
}

function fmtId(t) {
  return t.short_code != null ? `#${t.short_code}` : t.id.slice(0, 6);
}

function renderTension(t) {
  const li = document.createElement("li");
  li.className = "tension";
  if (t.overdue) li.classList.add("is-overdue");

  const id = document.createElement("span");
  id.className = "id";
  id.textContent = fmtId(t);

  const body = document.createElement("div");
  body.className = "body";

  const desired = document.createElement("div");
  desired.className = "desired";
  desired.textContent = t.desired;

  const actual = document.createElement("div");
  actual.className = "actual";
  actual.textContent = t.actual;

  const meta = document.createElement("div");
  meta.className = "meta";
  const bits = [];
  if (t.horizon) bits.push(t.horizon);
  if (t.position != null) bits.push(`pos ${t.position}`);
  meta.textContent = bits.join(" · ");

  body.appendChild(desired);
  body.appendChild(actual);
  if (bits.length) body.appendChild(meta);

  li.appendChild(id);
  li.appendChild(body);

  li.addEventListener("click", () => {
    if (API) window.location.href = `${API}/#/tension/${t.id}`;
  });

  return li;
}

function fillList(el, tensions) {
  el.replaceChildren(...tensions.map(renderTension));
}

function render(data) {
  bandOffline.hidden = true;

  const active = data.tensions.filter(
    (t) => (t.status || "").toLowerCase() === "active",
  );

  const overdue = active.filter((t) => t.overdue);
  const positioned = active
    .filter((t) => !t.overdue && t.position != null)
    .sort((a, b) => a.position - b.position)
    .slice(0, MAX_NEXT);
  const held = active
    .filter((t) => t.position == null && !t.overdue)
    .slice(0, MAX_HELD);

  bandOverdue.hidden = overdue.length === 0;
  bandNext.hidden = positioned.length === 0;
  bandHeld.hidden = held.length === 0;

  fillList(listOverdue, overdue);
  fillList(listNext, positioned);
  fillList(listHeld, held);

  const anySignal = !bandOverdue.hidden || !bandNext.hidden || !bandHeld.hidden;
  bandSilent.hidden = anySignal;

  const s = data.summary;
  summaryEl.textContent = `${s.active} active · ${s.resolved} resolved · ${s.released} released`;
  statusEl.textContent = `updated ${new Date().toLocaleTimeString()}`;
}

function renderOffline() {
  bandOverdue.hidden = true;
  bandNext.hidden = true;
  bandHeld.hidden = true;
  bandSilent.hidden = true;
  bandOffline.hidden = false;
  summaryEl.textContent = "";
  statusEl.textContent = "offline";
}

async function probePort(port) {
  const ctl = new AbortController();
  const timer = setTimeout(() => ctl.abort(), PROBE_TIMEOUT_MS);
  try {
    const res = await fetch(`http://localhost:${port}/api/tensions`, {
      cache: "no-store",
      signal: ctl.signal,
    });
    if (!res.ok) return null;
    return res;
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

async function discoverApi() {
  // Fast path: retry last known good port first.
  const order = [];
  if (lastGoodPort != null) order.push(lastGoodPort);
  for (let p = PORT_RANGE_START; p <= PORT_RANGE_END; p++) {
    if (p !== lastGoodPort) order.push(p);
  }

  for (const port of order) {
    const res = await probePort(port);
    if (res) {
      lastGoodPort = port;
      API = `http://localhost:${port}`;
      return res;
    }
  }
  API = null;
  return null;
}

async function load() {
  const res = await discoverApi();
  if (!res) {
    renderOffline();
    return;
  }
  try {
    const data = await res.json();
    render(data);
    refreshWorkspace();
  } catch (err) {
    console.warn("werk-tab: parse failed", err);
    renderOffline();
  }
}

async function refreshWorkspace() {
  if (!API) return;
  try {
    const res = await fetch(`${API}/api/workspaces`, { cache: "no-store" });
    if (!res.ok) return;
    const data = await res.json();
    currentWorkspace = data.current;
    workspaceList = data.available || [];
    renderWorkspaceHeader();
  } catch (err) {
    console.warn("werk-tab: workspace fetch failed", err);
  }
}

function renderWorkspaceHeader() {
  if (!currentWorkspace) {
    wsName.textContent = "—";
    return;
  }
  wsName.textContent = currentWorkspace.name;
  wsButton.title = currentWorkspace.path;
}

function renderWorkspaceMenu() {
  wsMenu.replaceChildren(
    ...workspaceList.map((ws) => {
      const li = document.createElement("li");
      li.setAttribute("role", "option");
      if (currentWorkspace && ws.path === currentWorkspace.path) {
        li.classList.add("is-current");
      }
      const name = document.createElement("span");
      name.className = "ws-name";
      name.textContent = ws.name;
      const path = document.createElement("span");
      path.className = "ws-path";
      path.textContent = ws.path;
      li.appendChild(name);
      li.appendChild(path);
      li.addEventListener("click", () => selectWorkspace(ws));
      return li;
    }),
  );
}

function toggleMenu(open) {
  const willOpen =
    open !== undefined ? open : wsButton.getAttribute("aria-expanded") !== "true";
  wsButton.setAttribute("aria-expanded", willOpen ? "true" : "false");
  wsMenu.hidden = !willOpen;
  if (willOpen) renderWorkspaceMenu();
}

async function selectWorkspace(ws) {
  if (switching) return;
  if (currentWorkspace && ws.path === currentWorkspace.path) {
    toggleMenu(false);
    return;
  }
  if (!API) return;
  switching = true;
  toggleMenu(false);
  wsName.textContent = `→ ${ws.name}`;
  try {
    await fetch(`${API}/api/workspace/select`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ path: ws.path }),
    });
  } catch (err) {
    console.warn("werk-tab: select failed", err);
  }
  // Daemon will exit and the supervisor restarts it. Wait, then reload.
  API = null;
  setTimeout(async () => {
    await load();
    switching = false;
  }, 1500);
}

wsButton.addEventListener("click", (e) => {
  e.stopPropagation();
  toggleMenu();
});

document.addEventListener("click", (e) => {
  if (!wsMenu.contains(e.target) && e.target !== wsButton) {
    toggleMenu(false);
  }
});

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") toggleMenu(false);
});

let es = null;
function connectStream() {
  if (!API) {
    setTimeout(connectStream, 5000);
    return;
  }
  try {
    es = new EventSource(`${API}/api/events`);
    es.onmessage = () => load();
    es.onerror = () => {
      if (es) {
        es.close();
        es = null;
      }
      // Port may have shifted after a daemon restart — rediscover.
      API = null;
      setTimeout(async () => {
        await load();
        connectStream();
      }, 5000);
    };
  } catch (err) {
    console.warn("werk-tab: SSE connect failed", err);
    setTimeout(connectStream, 5000);
  }
}

(async () => {
  await load();
  connectStream();
})();

document.addEventListener("visibilitychange", () => {
  if (document.visibilityState === "visible") load();
});

// Footer link needs to adapt to discovered port.
const footerLink = document.querySelector("footer .link");
if (footerLink) {
  footerLink.addEventListener("click", (e) => {
    if (API) {
      e.preventDefault();
      window.location.href = API;
    }
  });
}
