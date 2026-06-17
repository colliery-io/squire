// Minimal progressive-enhancement script for the Keep shell (ADR A-0008: light vanilla JS, no SPA
// framework). Posts the login form, and on success swaps to the authenticated shell. Later Keep
// tasks extend this with authoring/review/member/log views.
(function () {
  const form = document.getElementById("login-form");
  const errEl = document.getElementById("login-error");
  if (!form) return;

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
    document.getElementById("login").hidden = true;
    document.getElementById("who").textContent = `${who.display_name || "Knight"} (#${who.user})`;
    document.getElementById("shell").hidden = false;
    for (const id of ["review-panel", "quests-panel", "items-panel", "achievements-panel", "members-panel", "log-panel"]) {
      document.getElementById(id).hidden = false;
    }
    loadReview();
    loadQuests();
    loadCatalog("items", "item-list");
    loadCatalog("achievements", "achievement-list");
    loadMembers();
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

  // ── Generic catalog list/archive (items, achievements) ───────────────────────
  async function loadCatalog(kind, listId) {
    const res = await fetch(`/api/${kind}`);
    if (!res.ok) return;
    const rows = await res.json();
    const ul = document.getElementById(listId);
    ul.innerHTML = "";
    for (const row of rows) {
      const obj = row.item || row.achievement;
      const li = document.createElement("li");
      const label = row.item
        ? `${obj.name} — ${obj.cost} pts${obj.active ? "" : " (archived)"}${row.out_of_stock ? " · out of stock" : ""}`
        : `${obj.name}${obj.active ? "" : " (archived)"}`;
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
      document.getElementById("login").hidden = true;
      document.getElementById("who").textContent = `${who.display_name || "Admin"} (#${who.user})`;
      document.getElementById("shell").hidden = false;
      for (const id of ["review-panel", "quests-panel", "items-panel", "achievements-panel", "members-panel", "log-panel"]) {
        document.getElementById(id).hidden = false;
      }
      loadReview(); loadQuests(); loadCatalog("items", "item-list"); loadCatalog("achievements", "achievement-list"); loadMembers();
    });
  }

  // ── Quests (T-0026) ──────────────────────────────────────────────────────────
  // The session cookie is HttpOnly; same-origin fetch sends it automatically, so the Operator
  // extractor authenticates these calls without the page handling the token.
  const questForm = document.getElementById("quest-form");
  const questErr = document.getElementById("quest-error");

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
      li.textContent = `${q.title} — ${q.reward} pts${tag} `;
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
    questForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      questErr.hidden = true;
      const fd = new FormData(questForm);
      // A daily quest for all squires; each assignee does their own. The full
      // cadence/assignment/completion shape is available via the API for richer editors later.
      const quest = {
        // QuestId is a u128 deserialized from a JSON number; Date.now() is well under 2^53.
        id: Date.now(),
        title: fd.get("title"),
        description: null,
        category: null,
        reward: Number(fd.get("reward")),
        cadence: { Recurring: "Daily" },
        assignment: "AllSquires",
        completion: "EachAssignee",
        auto_approve: fd.get("auto_approve") === "on",
        repeatable_within_day: false,
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
      const item = {
        id: Date.now(),
        name: fd.get("name"),
        description: null,
        cost: Number(fd.get("cost")),
        gate: null,
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

  // ── Achievement create (T-0027) ──────────────────────────────────────────────
  const achForm = document.getElementById("achievement-form");
  const achErr = document.getElementById("achievement-error");
  if (achForm) {
    achForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      achErr.hidden = true;
      const fd = new FormData(achForm);
      const achievement = {
        id: Date.now(),
        name: fd.get("name"),
        description: null,
        criterion: { PointsEarned: { total: Number(fd.get("total")) } },
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
