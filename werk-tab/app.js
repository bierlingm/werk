// Canonical port range — must match `werk_shared::daemon_net::DEFAULT_PORT_RANGE`.
// Asserted by werk-cli/tests/port_range_parity.rs in CI.
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

const fieldToggle = $("field-toggle");
const fieldSection = $("field");
const fieldVitalsBody = $("field-vitals-body");
const fieldVitalsFoot = $("field-vitals-foot");
const fieldBandOverdue = $("field-band-overdue");
const fieldBandNext = $("field-band-next");
const fieldBandHeld = $("field-band-held");
const fieldBandSilent = $("field-band-silent");
const fieldListOverdue = $("field-list-overdue");
const fieldListNext = $("field-list-next");
const fieldListHeld = $("field-list-held");
const fieldSkipped = $("field-skipped");

let currentWorkspace = null;
let workspaceList = [];
let switching = false;
let fieldMode = false;

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
      updateFooterLink();
      return res;
    }
  }
  API = null;
  updateFooterLink();
  return null;
}

function updateFooterLink() {
  const footerLink = document.querySelector("footer .link");
  if (!footerLink) return;
  // When we know the API, make the href real so middle-click / copy-link-address
  // both work. When we don't, keep it inert.
  footerLink.href = API || "#";
}

async function load() {
  if (fieldMode) {
    await loadField();
    return;
  }
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

// ─── Field mode ────────────────────────────────────────────────────

async function loadField() {
  // Rediscover if API dropped.
  if (!API) {
    const res = await discoverApi();
    if (!res) {
      renderOffline();
      return;
    }
  }
  try {
    const [vitalsRes, attentionRes] = await Promise.all([
      fetch(`${API}/api/field/vitals`, { cache: "no-store" }),
      fetch(`${API}/api/field/attention`, { cache: "no-store" }),
    ]);
    if (!vitalsRes.ok || !attentionRes.ok) {
      renderOffline();
      return;
    }
    const vitals = await vitalsRes.json();
    const attention = await attentionRes.json();
    renderField(vitals, attention);
  } catch (err) {
    console.warn("werk-tab: field fetch failed", err);
    renderOffline();
  }
}

function renderField(vitals, attention) {
  bandOffline.hidden = true;

  // Per-space vitals table.
  fieldVitalsBody.replaceChildren(
    ...vitals.spaces.map((s) => {
      const tr = document.createElement("tr");
      tr.appendChild(tdText(s.name, "col-space"));
      tr.appendChild(tdText(String(s.active), "col-num"));
      const overdueCell = tdText(String(s.overdue), "col-num");
      if (s.overdue > 0) overdueCell.classList.add("is-overdue");
      tr.appendChild(overdueCell);
      tr.appendChild(tdText(String(s.positioned), "col-num"));
      tr.appendChild(tdText(String(s.held), "col-num"));
      tr.appendChild(tdText(fmtLast(s.last_activity), "col-time"));
      return tr;
    }),
  );

  const t = vitals.totals;
  const foot = document.createElement("tr");
  foot.classList.add("totals");
  foot.appendChild(tdText("total", "col-space"));
  foot.appendChild(tdText(String(t.active), "col-num"));
  const overdueTotal = tdText(String(t.overdue), "col-num");
  if (t.overdue > 0) overdueTotal.classList.add("is-overdue");
  foot.appendChild(overdueTotal);
  foot.appendChild(tdText(String(t.positioned), "col-num"));
  foot.appendChild(tdText(String(t.held), "col-num"));
  foot.appendChild(tdText("", "col-time"));
  fieldVitalsFoot.replaceChildren(foot);

  // Pooled bands.
  fillFieldBand(fieldBandOverdue, fieldListOverdue, attention.overdue, true);
  fillFieldBand(fieldBandNext, fieldListNext, attention.next_up, false);
  fillFieldBand(fieldBandHeld, fieldListHeld, attention.held, false);

  const anySignal =
    !fieldBandOverdue.hidden || !fieldBandNext.hidden || !fieldBandHeld.hidden;
  fieldBandSilent.hidden = anySignal;

  // Skipped spaces — one-line note, not a band.
  if (attention.skipped && attention.skipped.length > 0) {
    fieldSkipped.hidden = false;
    fieldSkipped.textContent = `skipped: ${attention.skipped
      .map((s) => `${s.name} (${s.reason})`)
      .join(", ")}`;
  } else {
    fieldSkipped.hidden = true;
    fieldSkipped.textContent = "";
  }

  summaryEl.textContent = `${vitals.spaces.length} space${
    vitals.spaces.length === 1 ? "" : "s"
  } · ${t.active} active`;
  statusEl.textContent = `updated ${new Date().toLocaleTimeString()}`;
}

function fillFieldBand(bandEl, listEl, items, overdue) {
  bandEl.hidden = items.length === 0;
  listEl.replaceChildren(
    ...items.map((item) => {
      const li = document.createElement("li");
      li.className = "tension field-item";
      if (overdue) li.classList.add("is-overdue");

      const tag = document.createElement("span");
      tag.className = "field-tag id";
      tag.textContent = `[${item.space_name}:${
        item.short_code != null ? `#${item.short_code}` : "?"
      }]`;

      const body = document.createElement("div");
      body.className = "body";

      const desired = document.createElement("div");
      desired.className = "desired";
      desired.textContent = item.desired;

      const bits = [];
      if (item.horizon) bits.push(`due ${item.horizon}`);
      else if (item.position != null) bits.push(`pos ${item.position}`);
      if (bits.length) {
        const meta = document.createElement("div");
        meta.className = "meta";
        meta.textContent = bits.join(" · ");
        body.appendChild(desired);
        body.appendChild(meta);
      } else {
        body.appendChild(desired);
      }

      li.appendChild(tag);
      li.appendChild(body);
      return li;
    }),
  );
}

function tdText(text, cls) {
  const td = document.createElement("td");
  if (cls) td.className = cls;
  td.textContent = text;
  return td;
}

function fmtLast(iso) {
  if (!iso) return "—";
  const then = new Date(iso);
  const now = new Date();
  const secs = Math.max(0, Math.floor((now - then) / 1000));
  if (secs < 60) return "just now";
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
  if (secs < 604800) return `${Math.floor(secs / 86400)}d ago`;
  return `${Math.floor(secs / 604800)}w ago`;
}

function toggleFieldMode(next) {
  const willEnter = next !== undefined ? next : !fieldMode;
  fieldMode = willEnter;
  fieldToggle.setAttribute("aria-pressed", willEnter ? "true" : "false");
  fieldToggle.classList.toggle("is-active", willEnter);
  fieldSection.hidden = !willEnter;
  // Hide space-mode bands in field mode.
  bandOverdue.hidden = willEnter ? true : bandOverdue.hidden;
  bandNext.hidden = willEnter ? true : bandNext.hidden;
  bandHeld.hidden = willEnter ? true : bandHeld.hidden;
  bandSilent.hidden = willEnter ? true : bandSilent.hidden;
  // Workspace switcher still renders — useful context showing the daemon's
  // active space even in field mode — but its menu stays usable either way.
  load();
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

fieldToggle.addEventListener("click", (e) => {
  e.stopPropagation();
  toggleFieldMode();
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

// Footer link: href is maintained by updateFooterLink() as API is discovered/lost.
// Normal click (target=_blank) opens a new tab; no custom handler needed.
