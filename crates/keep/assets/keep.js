// Minimal progressive-enhancement script for the Keep shell (ADR A-0008: light vanilla JS, no SPA
// framework). Posts the login form, and on success swaps to the authenticated shell. Later Keep
// tasks extend this with authoring/review/member/log views.
(function () {
  const form = document.getElementById("login-form");
  const errEl = document.getElementById("login-error");
  if (!form) return;

  // ── Tabbed shell (T-0061) ────────────────────────────────────────────────────
  // One panel visible at a time; the nav is a tab bar. Each tab maps to a panel and a loader
  // that re-pulls its data on activation (so Review etc. stay fresh). Function declarations
  // below are hoisted, so these loaders resolve even though they're defined later in the file.
  const TABS = [
    { tab: "quests", panel: "quests-panel" },
    { tab: "review", panel: "review-panel" },
    { tab: "rewards", panel: "items-panel" },
    { tab: "achievements", panel: "achievements-panel" },
    { tab: "members", panel: "members-panel" },
    { tab: "pair", panel: "pair-panel" },
    { tab: "log", panel: "log-panel" },
  ];
  const TAB_LOADERS = {
    quests: () => { loadQuestSquires().then(() => loadQuests()); },
    review: () => loadReview(),
    rewards: () => loadCatalog("items", "item-list"),
    achievements: () => { loadAchScopeQuests(); loadCatalog("achievements", "achievement-list"); },
    members: () => loadMembers(),
    pair: () => loadPairMembers(),
    log: () => {},
  };

  function currentTab() {
    const name = (location.hash || "").replace("#", "");
    return TABS.some((t) => t.tab === name) ? name : "quests";
  }

  function showTab(name) {
    if (!TABS.some((t) => t.tab === name)) name = "quests";
    for (const t of TABS) document.getElementById(t.panel).hidden = t.tab !== name;
    for (const a of document.querySelectorAll("#tabs .tab")) {
      a.classList.toggle("active", a.dataset.tab === name);
    }
    (TAB_LOADERS[name] || (() => {}))();
  }

  // Reveal the authenticated shell and land on the active tab. Shared by login + first-run
  // register. Eager-loads only the catalogs that feed cross-tab dropdowns (item gate ←
  // achievements, achievement scope ← quests); each tab loads its own data on activation.
  function enterShell(who, fallbackName) {
    document.getElementById("login").hidden = true;
    document.getElementById("who").textContent = `${who.display_name || fallbackName} (#${who.user})`;
    document.getElementById("shell").hidden = false;
    loadAchScopeQuests();
    loadCatalog("achievements", "achievement-list");
    showTab(currentTab());
  }

  window.addEventListener("hashchange", () => {
    if (!document.getElementById("shell").hidden) showTab(currentTab());
  });

  form.addEventListener("submit", async (ev) => {
    ev.preventDefault();
    errEl.hidden = true;
    const body = new URLSearchParams(new FormData(form));
    const res = await fetch("/login", {
      method: "POST",
      headers: { "content-type": "application/x-www-form-urlencoded" },
      body,
    });
    if (!res.ok) {
      errEl.textContent = res.status === 403 ? "Only Knights can operate the Keep." : "Sign-in failed.";
      errEl.hidden = false;
      return;
    }
    const who = await res.json();
    enterShell(who, "Knight");
  });

  // ── Review queue (T-0029) ────────────────────────────────────────────────────
  async function reviewAction(path, body) {
    await fetch(path, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
    loadReview();
  }

  async function loadReview() {
    const res = await fetch("/api/review");
    if (!res.ok) return;
    const r = await res.json();
    const claims = document.getElementById("claim-queue");
    const reqs = document.getElementById("request-queue");
    const sqs = document.getElementById("squire-balances");
    claims.innerHTML = "";
    reqs.innerHTML = "";
    sqs.innerHTML = "";
    document.getElementById("review-empty").hidden = r.pending_claims.length + r.pending_requests.length > 0;

    for (const c of r.pending_claims) {
      const li = document.createElement("li");
      li.textContent = `${c.quest_title} — squire #${c.squire} `;
      const ok = document.createElement("button");
      ok.textContent = "Approve";
      ok.addEventListener("click", () => reviewAction("/api/review/claim", { claim_id: c.claim_id, decision: "approve" }));
      const no = document.createElement("button");
      no.textContent = "Reject";
      no.addEventListener("click", () => reviewAction("/api/review/claim", { claim_id: c.claim_id, decision: { reject: { reason: prompt("Reason?") || null } } }));
      li.append(ok, no);
      claims.appendChild(li);
    }
    for (const q of r.pending_requests) {
      const li = document.createElement("li");
      li.textContent = `${q.item_name} (${q.cost} pts) — squire #${q.squire} `;
      const ok = document.createElement("button");
      ok.textContent = "Approve";
      ok.addEventListener("click", () => reviewAction("/api/review/redemption", { request_id: q.request_id, decision: "approve" }));
      const no = document.createElement("button");
      no.textContent = "Reject";
      no.addEventListener("click", () => reviewAction("/api/review/redemption", { request_id: q.request_id, decision: { reject: { reason: prompt("Reason?") || null } } }));
      li.append(ok, no);
      reqs.appendChild(li);
    }
    for (const s of r.squires) {
      const li = document.createElement("li");
      li.textContent = `${s.display_name}: ${s.balance} pts `;
      const adj = document.createElement("button");
      adj.textContent = "Adjust";
      adj.addEventListener("click", () => {
        const amount = Number(prompt("Adjust by (+/-):"));
        const reason = prompt("Reason (required):");
        if (!reason) return;
        reviewAction("/api/adjust", { command_id: Date.now(), squire: s.squire, amount, reason });
      });
      li.appendChild(adj);
      sqs.appendChild(li);
    }
  }

  // ── Members (T-0028) ─────────────────────────────────────────────────────────
  async function loadMembers() {
    const res = await fetch("/api/members");
    if (!res.ok) return;
    const rows = await res.json();
    const ul = document.getElementById("member-list");
    ul.innerHTML = "";
    for (const m of rows) {
      const li = document.createElement("li");
      li.textContent = `${m.display_name} — ${m.role}${m.active ? "" : " (inactive)"} `;
      const btn = document.createElement("button");
      btn.textContent = m.active ? "Deactivate" : "Reactivate";
      btn.addEventListener("click", async () => {
        await fetch(`/api/members/${m.user}/active`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ active: !m.active }),
        });
        loadMembers();
      });
      li.appendChild(btn);
      ul.appendChild(li);
    }
  }

  const memberForm = document.getElementById("member-form");
  const memberErr = document.getElementById("member-error");
  const memberTok = document.getElementById("member-token");
  if (memberForm) {
    memberForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      memberErr.hidden = true;
      memberTok.hidden = true;
      const fd = new FormData(memberForm);
      const res = await fetch("/api/members", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          role: fd.get("role"),
          display_name: fd.get("display_name"),
          initial_secret: fd.get("initial_secret"),
        }),
      });
      if (!res.ok) {
        memberErr.textContent = "Could not add member.";
        memberErr.hidden = false;
        return;
      }
      const added = await res.json();
      memberTok.textContent = `Pairing token for ${fd.get("display_name")}: ${added.token}`;
      memberTok.hidden = false;
      memberForm.reset();
      loadMembers();
    });
  }

  // ── Achievement helpers (T-0036) ─────────────────────────────────────────────
  // The achievements list, kept around so the item list can resolve a gate id → name and the
  // item-gate dropdown can be (re)populated.
  let achievementRows = [];

  // Render a Scope (externally-tagged serde JSON) as a short human label.
  function scopeLabel(scope) {
    if (scope === "Any") return "Any";
    if (scope && scope.Quest !== undefined) {
      const row = (window.__questsById && window.__questsById[scope.Quest]) || null;
      return row ? `Quest:${row}` : `Quest #${scope.Quest}`;
    }
    if (scope && scope.Category !== undefined) return `Category:${scope.Category}`;
    return "?";
  }

  // Summarize a Criterion (externally-tagged serde JSON) for the list view.
  function criterionSummary(c) {
    if (c && c.Streak) {
      return `Streak · ${scopeLabel(c.Streak.scope)} · ${c.Streak.length} (${c.Streak.basis})`;
    }
    if (c && c.TotalCompletions) {
      return `TotalCompletions · ${scopeLabel(c.TotalCompletions.scope)} · ${c.TotalCompletions.count}`;
    }
    if (c && c.PointsEarned) {
      return `PointsEarned · ${c.PointsEarned.total}`;
    }
    return "?";
  }

  // Resolve a gate (achievement id) → its name, falling back to the raw id.
  function gateLabel(gateId) {
    const row = achievementRows.find((r) => r.achievement.id === gateId);
    return row ? row.achievement.name : `#${gateId}`;
  }

  // ── Generic catalog list/archive (items, achievements) ───────────────────────
  async function loadCatalog(kind, listId) {
    const res = await fetch(`/api/${kind}`);
    if (!res.ok) return;
    const rows = await res.json();
    if (kind === "achievements") {
      achievementRows = rows;
      populateItemGate();
    }
    const ul = document.getElementById(listId);
    ul.innerHTML = "";
    for (const row of rows) {
      const obj = row.item || row.achievement;
      const li = document.createElement("li");
      let label;
      if (row.item) {
        const gate = obj.gate != null ? ` · needs: ${gateLabel(obj.gate)}` : "";
        label = `${obj.name} — ${obj.cost} pts${obj.active ? "" : " (archived)"}${row.out_of_stock ? " · out of stock" : ""}${gate}`;
      } else {
        label = `${obj.name} — ${criterionSummary(obj.criterion)} · +${obj.bonus_points}${obj.active ? "" : " (archived)"}`;
      }
      li.textContent = label + " ";
      if (obj.active) {
        const btn = document.createElement("button");
        btn.textContent = "Archive";
        btn.addEventListener("click", async () => {
          await fetch(`/api/${kind}/${obj.id}/archive`, { method: "POST" });
          loadCatalog(kind, listId);
        });
        li.appendChild(btn);
      }
      ul.appendChild(li);
    }
  }

  // Populate the item-form gate <select> (None + each achievement) from the loaded rows.
  function populateItemGate() {
    const sel = document.getElementById("item-gate");
    if (!sel) return;
    const prev = sel.value;
    sel.innerHTML = '<option value="">None</option>';
    for (const r of achievementRows) {
      const opt = document.createElement("option");
      opt.value = String(r.achievement.id);
      opt.textContent = r.achievement.name;
      sel.appendChild(opt);
    }
    sel.value = prev;
  }

  // ── First-run registration ───────────────────────────────────────────────────
  const registerForm = document.getElementById("register-form");
  const registerErr = document.getElementById("register-error");
  if (registerForm) {
    registerForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      registerErr.hidden = true;
      const res = await fetch("/register", {
        method: "POST",
        headers: { "content-type": "application/x-www-form-urlencoded" },
        body: new URLSearchParams(new FormData(registerForm)),
      });
      if (!res.ok) {
        registerErr.textContent = res.status === 409 ? "Already set up — just sign in." : "Could not create admin.";
        registerErr.hidden = false;
        return;
      }
      const who = await res.json();
      enterShell(who, "Admin");
    });
  }

  // ── Device pairing (ADR A-0010 / T-0045) ─────────────────────────────────────
  // Populate the member dropdown, then mint a one-time code + QR for the chosen member.
  async function loadPairMembers() {
    const sel = document.getElementById("pair-member");
    if (!sel) return;
    const res = await fetch("/api/members");
    if (!res.ok) return;
    const rows = await res.json();
    sel.innerHTML = "";
    for (const m of rows.filter((m) => m.active)) {
      const opt = document.createElement("option");
      opt.value = m.user;
      opt.textContent = `${m.display_name} — ${m.role}`;
      sel.appendChild(opt);
    }
  }

  const pairForm = document.getElementById("pair-form");
  if (pairForm) {
    pairForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const err = document.getElementById("pair-error");
      const result = document.getElementById("pair-result");
      err.hidden = true;
      result.hidden = true;
      const user = document.getElementById("pair-member").value;
      const res = await fetch("/api/pair/codes", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ user }),
      });
      if (!res.ok) {
        err.textContent = "Could not mint a pairing code.";
        err.hidden = false;
        return;
      }
      const p = await res.json();
      document.getElementById("pair-qr").innerHTML = p.qr_svg;
      document.getElementById("pair-code").textContent = p.code;
      const mins = Math.max(0, Math.round((p.expires_at - Date.now()) / 60000));
      document.getElementById("pair-expiry").textContent =
        `Reaches ${p.host}:${p.port} · household "${p.household}" · expires in ~${mins} min (single use).`;
      result.hidden = false;
    });
  }

  // ── Quests (T-0026 / rich authoring T-0062) ──────────────────────────────────
  // The session cookie is HttpOnly; same-origin fetch sends it automatically, so the Operator
  // extractor authenticates these calls without the page handling the token.
  const questForm = document.getElementById("quest-form");
  const questErr = document.getElementById("quest-error");

  // Active squires, for the assignment picker + list labels.
  let squiresById = {};
  async function loadQuestSquires() {
    const res = await fetch("/api/members");
    if (!res.ok) return;
    const rows = await res.json();
    squiresById = {};
    const box = document.getElementById("quest-squires");
    if (box) box.innerHTML = "";
    for (const m of rows.filter((m) => m.role === "Squire" && m.active)) {
      squiresById[m.user] = m.display_name;
      if (box) {
        const lbl = document.createElement("label");
        const cb = document.createElement("input");
        cb.type = "checkbox"; cb.name = "squire"; cb.value = String(m.user);
        lbl.append(cb, document.createTextNode(" " + m.display_name));
        box.appendChild(lbl);
      }
    }
  }

  // yyyy-mm-dd → the domain's Monday-aligned Date day-count (floor(unixDays)+3, matching
  // store::date_from_unix_millis). Returns null on a blank/invalid date.
  function toDomainDate(str) {
    const ms = Date.parse(str + "T00:00:00Z");
    if (Number.isNaN(ms)) return null;
    return Math.floor(ms / 86400000) + 3;
  }

  // Short human summary of a quest's cadence + assignment for the list view.
  function questSummary(q) {
    let cadence = "One-time";
    const c = q.cadence || {};
    if (c.Recurring === "Daily") cadence = "Daily";
    else if (c.Recurring && c.Recurring.Weekly) cadence = c.Recurring.Weekly.days.join("/");
    let who = "all squires";
    if (q.assignment && q.assignment.Squires) {
      who = q.assignment.Squires.map((id) => squiresById[id] || `#${id}`).join(", ");
    }
    const extras = [];
    if (q.completion === "Race") extras.push("race");
    if (q.repeatable_within_day) extras.push("repeatable/day");
    if (q.auto_approve) extras.push("auto-approve");
    return `${cadence} · ${who}${extras.length ? " · " + extras.join(", ") : ""}`;
  }

  // Progressive disclosure: weekday picker for Weekly, due field for One-time, squire list for "some".
  function syncQuestFields() {
    const cadence = document.getElementById("quest-cadence").value;
    document.getElementById("quest-weekly-fields").hidden = cadence !== "Weekly";
    document.getElementById("quest-due-label").hidden = cadence !== "OneOff";
    const checked = questForm.querySelector('input[name="assign"]:checked');
    document.getElementById("quest-squires").hidden = !(checked && checked.value === "some");
  }

  async function loadQuests() {
    const res = await fetch("/api/quests");
    if (!res.ok) return;
    const rows = await res.json();
    const ul = document.getElementById("quest-list");
    ul.innerHTML = "";
    for (const row of rows) {
      const q = row.quest;
      const li = document.createElement("li");
      const tag = q.active ? "" : " (archived)";
      const strong = document.createElement("strong");
      strong.textContent = q.title;
      li.append(strong, document.createTextNode(` — ${q.reward} ★ · ${questSummary(q)}${tag} `));
      if (q.active) {
        const btn = document.createElement("button");
        btn.textContent = "Archive";
        btn.addEventListener("click", async () => {
          await fetch(`/api/quests/${q.id}/archive`, { method: "POST" });
          loadQuests();
        });
        li.appendChild(btn);
      }
      ul.appendChild(li);
    }
  }

  if (questForm) {
    document.getElementById("quest-cadence").addEventListener("change", syncQuestFields);
    for (const r of questForm.querySelectorAll('input[name="assign"]')) {
      r.addEventListener("change", syncQuestFields);
    }
    syncQuestFields();

    questForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      questErr.hidden = true;
      const fd = new FormData(questForm);

      // Cadence — externally-tagged enum JSON (Daily | {Weekly:{days}} | {OneOff:{due}}).
      let cadence;
      const cad = fd.get("cadence");
      if (cad === "Daily") {
        cadence = { Recurring: "Daily" };
      } else if (cad === "Weekly") {
        const days = fd.getAll("wd");
        if (days.length === 0) { questErr.textContent = "Pick at least one weekday."; questErr.hidden = false; return; }
        cadence = { Recurring: { Weekly: { days } } };
      } else {
        const dueStr = fd.get("due");
        cadence = { OneOff: { due: dueStr ? toDomainDate(dueStr) : null } };
      }

      // Assignment — "AllSquires" or {Squires:[ids]} (must be ≥1 active squire).
      let assignment = "AllSquires";
      if (fd.get("assign") === "some") {
        const ids = fd.getAll("squire").map(Number);
        if (ids.length === 0) { questErr.textContent = "Pick at least one squire, or choose All squires."; questErr.hidden = false; return; }
        assignment = { Squires: ids };
      }

      const category = (fd.get("category") || "").trim();
      const quest = {
        // QuestId is a u128 deserialized from a JSON number; Date.now() is well under 2^53.
        id: Date.now(),
        title: fd.get("title"),
        description: null,
        category: category || null,
        reward: Number(fd.get("reward")),
        cadence,
        assignment,
        completion: fd.get("completion") || "EachAssignee",
        auto_approve: fd.get("auto_approve") === "on",
        repeatable_within_day: fd.get("repeat_day") === "on",
        active: true,
        icon: null,
      };
      const res = await fetch("/api/quests", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(quest),
      });
      if (!res.ok) {
        questErr.textContent = res.status === 400 ? "Invalid quest." : "Could not save quest.";
        questErr.hidden = false;
        return;
      }
      questForm.reset();
      syncQuestFields();
      loadQuests();
    });
  }

  // ── Item create (T-0027) ─────────────────────────────────────────────────────
  const itemForm = document.getElementById("item-form");
  const itemErr = document.getElementById("item-error");
  if (itemForm) {
    itemForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      itemErr.hidden = true;
      const fd = new FormData(itemForm);
      const gateRaw = fd.get("gate");
      const item = {
        id: Date.now(),
        name: fd.get("name"),
        description: null,
        cost: Number(fd.get("cost")),
        // gate is an achievement id number, or null for "None".
        gate: gateRaw ? Number(gateRaw) : null,
        availability: fd.get("availability"), // "Once" | "Repeatable"
        active: true,
        icon: null,
      };
      const res = await fetch("/api/items", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(item),
      });
      if (!res.ok) {
        itemErr.textContent = "Could not save reward.";
        itemErr.hidden = false;
        return;
      }
      itemForm.reset();
      loadCatalog("items", "item-list");
    });
  }

  // ── Achievement create (T-0027 / enriched T-0036) ────────────────────────────
  const achForm = document.getElementById("achievement-form");
  const achErr = document.getElementById("achievement-error");

  // Show only the fields relevant to the chosen criterion / scope. Driven by the two selects;
  // re-run on change and once at startup.
  function syncAchFields() {
    const criterion = document.getElementById("ach-criterion").value;
    const isPoints = criterion === "PointsEarned";
    const isTotal = criterion === "TotalCompletions";
    const isStreak = criterion === "Streak";
    document.getElementById("ach-points-fields").hidden = !isPoints;
    document.getElementById("ach-scope-fields").hidden = isPoints; // scope for Streak + TotalCompletions
    document.getElementById("ach-total-fields").hidden = !isTotal;
    document.getElementById("ach-streak-fields").hidden = !isStreak;

    const scope = document.getElementById("ach-scope").value;
    document.getElementById("ach-scope-quest-label").hidden = scope !== "Quest";
    document.getElementById("ach-scope-category-label").hidden = scope !== "Category";
  }

  // Populate the scope→Quest dropdown from GET /api/quests (options = quest title valued by id).
  async function loadAchScopeQuests() {
    const res = await fetch("/api/quests");
    if (!res.ok) return;
    const rows = await res.json();
    window.__questsById = {};
    const sel = document.getElementById("ach-scope-quest");
    if (sel) sel.innerHTML = "";
    for (const row of rows) {
      window.__questsById[row.quest.id] = row.quest.title;
      if (sel) {
        const opt = document.createElement("option");
        opt.value = String(row.quest.id);
        opt.textContent = row.quest.title;
        sel.appendChild(opt);
      }
    }
  }

  // Build the externally-tagged Scope JSON from the scope sub-control.
  function buildScope(fd) {
    const scope = fd.get("scope");
    if (scope === "Quest") return { Quest: Number(fd.get("scope_quest")) };
    if (scope === "Category") return { Category: fd.get("scope_category") || "" };
    return "Any";
  }

  if (achForm) {
    document.getElementById("ach-criterion").addEventListener("change", syncAchFields);
    document.getElementById("ach-scope").addEventListener("change", syncAchFields);
    syncAchFields();

    achForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      achErr.hidden = true;
      const fd = new FormData(achForm);
      const kind = fd.get("criterion");
      let criterion;
      if (kind === "PointsEarned") {
        criterion = { PointsEarned: { total: Number(fd.get("total")) } };
      } else if (kind === "TotalCompletions") {
        criterion = { TotalCompletions: { scope: buildScope(fd), count: Number(fd.get("count")) } };
      } else {
        criterion = { Streak: { scope: buildScope(fd), length: Number(fd.get("length")), basis: fd.get("basis") } };
      }
      const achievement = {
        id: Date.now(),
        name: fd.get("name"),
        description: null,
        criterion,
        bonus_points: Number(fd.get("bonus")),
        active: true,
      };
      const res = await fetch("/api/achievements", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(achievement),
      });
      if (!res.ok) {
        achErr.textContent = res.status === 400 ? "Invalid achievement." : "Could not save achievement.";
        achErr.hidden = false;
        return;
      }
      achForm.reset();
      syncAchFields();
      loadCatalog("achievements", "achievement-list");
    });
  }

  // ── Event-log inspector (T-0030) ─────────────────────────────────────────────
  const logForm = document.getElementById("log-form");
  if (logForm) {
    logForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const fd = new FormData(logForm);
      const res = await fetch(`/api/log/${fd.get("scope")}/${fd.get("id")}`);
      const out = document.getElementById("log-output");
      out.textContent = res.ok ? JSON.stringify(await res.json(), null, 2) : `Error ${res.status}`;
    });
  }
})();
