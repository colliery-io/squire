// Minimal progressive-enhancement script for the Keep shell (ADR A-0008: light vanilla JS, no SPA
// framework). Posts the login form, and on success swaps to the authenticated shell. Later Keep
// tasks extend this with authoring/review/member/log views.
(function () {
  const form = document.getElementById("login-form");
  const errEl = document.getElementById("login-error");
  if (!form) return;

  // ── Redesign card helpers (medallion · body · coin · chip). Pure presentation;
  //    all data, fetches and handlers below are unchanged. ─────────────────────
  function el(tag, cls, text) {
    const n = document.createElement(tag);
    if (cls) n.className = cls;
    if (text != null) n.textContent = text;
    return n;
  }
  function pickEmoji(text, table, fallback) {
    const t = text || "";
    for (const [re, e] of table) if (re.test(t)) return e;
    return fallback;
  }
  // The tinctures a squire may choose (mirrors `contract::TINCTURES`; hex mirrors the Android theme).
  const TINCTURES = ["gules", "azure", "vert", "purpure", "tenne", "sable"];
  const TINCTURE_HEX = { gules: "#C22B3A", azure: "#2457C5", vert: "#187A4F", purpure: "#6B3FA0", tenne: "#B8561B", sable: "#2E3440" };

  // "today" / "yesterday" / "Mon 15 Sep" from the server's day numbers — never "day 20624".
  function whenLabel(on, today) {
    const d = today - on;
    if (d === 0) return "today";
    if (d === 1) return "yesterday";
    return new Date(on * 86400000).toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  }

  // Reveal a form that lives behind a "New …" toggle (SQUIRE-T-0135 follow-up); no-op if already open.
  function openCompose(details) { if (details && !details.open) details.open = true; }

  const QUEST_EMOJI = [
    [/bed|wake|morning/i, "🛏"], [/tidy|room|clean|vacuum|dust/i, "🧹"],
    [/dog|walk|pet|biscuit|cat|feed/i, "🐕"], [/piano|music|practice|guitar|violin/i, "🎹"],
    [/homework|study|read|school|book/i, "📚"], [/table|dish|kitchen|wash|plate/i, "🍽"],
    [/trash|recycl|bin|garbage|compost/i, "♻"], [/laundry|fold|fluff/i, "🧺"],
    [/tooth|brush|teeth/i, "🪥"], [/water|plant|garden|flower|leaf/i, "🪴"],
  ];
  const REWARD_EMOJI = [
    [/ice ?cream|sundae|gelato/i, "🍦"], [/movie|film|cinema/i, "🎬"],
    [/screen|game|video|tv|console/i, "🎮"], [/park|outing|trip|zoo|outside/i, "🌳"],
    [/pizza|dinner|treat|candy|snack|dessert/i, "🍕"], [/book|read|story|comic/i, "📖"],
    [/money|allowance|cash|coin/i, "💰"], [/toy|lego|figure/i, "🧸"],
    [/sleep|stay ?up|bed ?time|late/i, "🌙"],
  ];
  function questEmoji(q) { return q.icon || pickEmoji((q.title || "") + " " + (q.category || ""), QUEST_EMOJI, "📜"); }
  function rewardEmoji(it) { return it.icon || pickEmoji(it.name || "", REWARD_EMOJI, "🎁"); }
  function medallion(emoji) { return el("span", "medallion", emoji); }
  function coinPill(amount) {
    const c = el("span", "coin");
    c.innerHTML = '<span class="disc">★</span>';
    c.appendChild(el("span", "amt", String(amount)));
    return c;
  }
  function cardBody(title, metaArr) {
    const body = el("div", "grow");
    body.appendChild(el("div", "card-title", title));
    if (metaArr && metaArr.length) {
      const meta = el("div", "card-meta");
      metaArr.forEach((p, i) => {
        if (i) meta.appendChild(el("span", "sep", "·"));
        meta.appendChild(el("span", i === 0 ? "lead" : null, p));
      });
      body.appendChild(meta);
    }
    return body;
  }

  // ── Tabbed shell (T-0061) ────────────────────────────────────────────────────
  // One panel visible at a time; the nav is a tab bar. Each tab maps to a panel and a loader
  // that re-pulls its data on activation (so Review etc. stay fresh). Function declarations
  // below are hoisted, so these loaders resolve even though they're defined later in the file.
  const TABS = [
    { tab: "quests", panel: "quests-panel" },
    { tab: "review", panel: "review-panel" },
    { tab: "rewards", panel: "items-panel" },
    { tab: "achievements", panel: "achievements-panel" },
    { tab: "hazards", panel: "hazards-panel" },
    { tab: "members", panel: "members-panel" },
    { tab: "pair", panel: "pair-panel" },
    { tab: "log", panel: "log-panel" },
    { tab: "settings", panel: "settings-panel" },
  ];
  const TAB_LOADERS = {
    quests: () => { loadQuestSquires().then(() => loadQuests()); },
    review: () => loadReview(),
    rewards: () => { loadCatalog("items", "item-list"); loadRewardsLibrary(); },
    achievements: () => { loadAchScopeQuests(); loadCatalog("achievements", "achievement-list"); loadAchLibrary(); },
    hazards: () => loadHazards(),
    members: () => loadMembers(),
    pair: () => { loadPairMembers(); loadInstallQr(); },
    log: () => {},
    settings: () => loadSettings(),
  };

  function currentTab() {
    const name = (location.hash || "").replace("#", "");
    return TABS.some((t) => t.tab === name) ? name : landingTab;
  }
  // Where the Keep opens with no hash (SQUIRE-T-0135): Review when anything is waiting, else Quests.
  let landingTab = "quests";
  async function refreshReviewCount() {
    try {
      const r = await fetch("/api/review");
      if (!r.ok) return 0;
      const v = await r.json();
      const n = (v.pending_claims || []).length + (v.pending_requests || []).length + (v.pending_cashouts || []).length;
      const badge = document.getElementById("review-count");
      badge.textContent = n;
      badge.hidden = n === 0;
      landingTab = n > 0 ? "review" : "quests";
      return n;
    } catch (_) { return 0; }
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
  async function enterShell(who, fallbackName) {
    document.getElementById("login").hidden = true;
    document.getElementById("who").textContent = `${who.display_name || fallbackName} (#${who.user})`;
    document.getElementById("shell").hidden = false;
    loadAchScopeQuests();
    loadCatalog("achievements", "achievement-list");
    loadLibrary();
    await refreshReviewCount(); // decides the landing tab when there is no hash
    showTab(currentTab());
  }

  window.addEventListener("hashchange", () => {
    if (!document.getElementById("shell").hidden) showTab(currentTab());
  });

  // Restore the session on load. The `keep_session` cookie outlives the page, but the shell used
  // to be revealed only by the login form's submit handler — so a refresh (or reopening the tab)
  // dumped a signed-in Knight back at the login form. Ask the server who we are first.
  (async () => {
    try {
      const res = await fetch("/api/whoami");
      if (res.ok) enterShell(await res.json(), "Knight");
    } catch (_) { /* offline or server down: the login form is already showing */ }
  })();

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
      errEl.textContent =
        res.status === 403 ? "Only Knights can operate the Keep."
        : res.status === 429 ? "Too many failed attempts — this account is locked for a while. Try again later."
        : "Sign-in failed.";
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
    refreshReviewCount();
  }
  // Seal all (SQUIRE-T-0135): every pending claim + request, one tap. Same calls the per-row
  // buttons make, fired together, one reload at the end.
  document.getElementById("seal-all").addEventListener("click", async () => {
    const calls = [];
    for (const c of pendingClaims) calls.push(fetch("/api/review/claim", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ claim_id: c.claim_id, decision: "approve" }) }));
    for (const q of pendingRequests) calls.push(fetch("/api/review/redemption", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ request_id: q.request_id, decision: "approve" }) }));
    await Promise.all(calls);
    loadReview();
    refreshReviewCount();
  });
  let pendingClaims = [], pendingRequests = [];

  // An inline "Reject — type a reason" editor (T-0080): replaces the old browser prompt(). Reveals a
  // reason field + Confirm/Cancel in place; Confirm posts the reject with the typed reason (optional).
  function attachRejectEditor(li, rejectBtn, onConfirm) {
    rejectBtn.addEventListener("click", () => {
      rejectBtn.hidden = true;
      const editor = document.createElement("span");
      editor.className = "inline-editor";
      const input = document.createElement("input");
      input.type = "text";
      input.placeholder = "Reason (optional — the child sees this)";
      input.className = "reject-reason";
      const confirm = document.createElement("button");
      confirm.textContent = "Send";
      confirm.className = "confirm-reject";
      const cancel = document.createElement("button");
      cancel.textContent = "Cancel";
      const close = () => { editor.remove(); rejectBtn.hidden = false; };
      confirm.addEventListener("click", () => { onConfirm(input.value.trim() || null); });
      cancel.addEventListener("click", close);
      editor.append(input, confirm, cancel);
      li.appendChild(editor);
      input.focus();
    });
  }

  async function loadReview() {
    const res = await fetch("/api/review");
    if (!res.ok) return;
    const r = await res.json();
    // The review carries each squire's name (and tincture): never "Squire #2" (SQUIRE-T-0135).
    for (const s of r.squires || []) squiresById[s.squire] = s.display_name;
    const claims = document.getElementById("claim-queue");
    const reqs = document.getElementById("request-queue");
    const sqs = document.getElementById("squire-balances");
    claims.innerHTML = "";
    reqs.innerHTML = "";
    sqs.innerHTML = "";
    document.getElementById("review-empty").hidden = r.pending_claims.length + r.pending_requests.length > 0;
    pendingClaims = r.pending_claims; pendingRequests = r.pending_requests;
    document.getElementById("review-actions").hidden = r.pending_claims.length + r.pending_requests.length < 2;

    for (const c of r.pending_claims) {
      const li = el("li", "review-card");
      const rowEl = el("div", "card-row");
      rowEl.appendChild(medallion(pickEmoji(c.quest_title, QUEST_EMOJI, "📜")));
      const who = squiresById[c.squire] || ("Squire #" + c.squire);
      rowEl.appendChild(cardBody(c.quest_title, [who, whenLabel(c.on, r.today)]));
      if (c.reward > 0) rowEl.appendChild(coinPill(c.reward));
      li.appendChild(rowEl);
      const actions = el("div", "card-actions");
      const ok = el("button", "approve", "Seal it");
      ok.addEventListener("click", () => reviewAction("/api/review/claim", { claim_id: c.claim_id, decision: "approve" }));
      const no = el("button", "reject", "Not yet…");
      attachRejectEditor(li, no, (reason) =>
        reviewAction("/api/review/claim", { claim_id: c.claim_id, decision: { reject: { reason } } }));
      actions.append(ok, no);
      li.appendChild(actions);
      claims.appendChild(li);
    }
    for (const q of r.pending_requests) {
      const li = el("li", "review-card");
      const rowEl = el("div", "card-row");
      rowEl.appendChild(medallion(pickEmoji(q.item_name, REWARD_EMOJI, "🎁")));
      const who = squiresById[q.squire] || ("Squire #" + q.squire);
      rowEl.appendChild(cardBody(q.item_name, [who, "wants to redeem · " + q.cost + " ★"]));
      li.appendChild(rowEl);
      const actions = el("div", "card-actions");
      const ok = el("button", "approve", "Grant it");
      ok.addEventListener("click", () => reviewAction("/api/review/redemption", { request_id: q.request_id, decision: "approve" }));
      const no = el("button", "reject", "Not yet…");
      attachRejectEditor(li, no, (reason) =>
        reviewAction("/api/review/redemption", { request_id: q.request_id, decision: { reject: { reason } } }));
      actions.append(ok, no);
      li.appendChild(actions);
      reqs.appendChild(li);
    }
    for (const s of r.squires) {
      const li = el("li", "list-row");
      const av = el("span", "medallion avatar-green", (s.display_name || "?").slice(0, 1).toUpperCase());
      if (s.tincture && TINCTURE_HEX[s.tincture]) av.style.setProperty("--t", TINCTURE_HEX[s.tincture]);
      const info = el("div", "grow");
      info.appendChild(el("div", "card-title", s.display_name));
      const owed = Number(s.cash_balance || 0);
      info.appendChild(el("div", "card-meta", owed > 0 ? `Squire · $${owed} owed` : "Squire"));
      const coin = coinPill(s.balance);
      const adj = el("button", "btn-sm btn-ghost", "Adjust");
      // Pay out real money owed (SQUIRE-T-0099): a negative Cash adjust. Only shown when $ is owed.
      let pay = null;
      if (owed > 0) {
        pay = el("button", "btn-sm btn-ghost", `Pay $${owed}`);
        pay.addEventListener("click", () => {
          if (!confirm(`Mark $${owed} paid to ${s.display_name}? This clears what's owed.`)) return;
          reviewAction("/api/adjust", { command_id: Date.now(), squire: s.squire, currency: "Cash", amount: -owed, reason: "Paid out" });
        });
      }
      // Inline adjust editor (T-0080): amount (±) + a required reason; Apply stays disabled until the
      // reason is non-blank (the engine 400s an empty reason).
      adj.addEventListener("click", () => {
        adj.hidden = true;
        const editor = document.createElement("span");
        editor.className = "inline-editor";
        const amount = document.createElement("input");
        amount.type = "number";
        amount.placeholder = "± points";
        amount.className = "adjust-amount";
        amount.value = "5";
        const reason = document.createElement("input");
        reason.type = "text";
        reason.placeholder = "Reason (required)";
        reason.className = "adjust-reason";
        const apply = document.createElement("button");
        apply.textContent = "Apply";
        apply.className = "apply-adjust";
        apply.disabled = true;
        const cancel = document.createElement("button");
        cancel.textContent = "Cancel";
        reason.addEventListener("input", () => { apply.disabled = reason.value.trim() === ""; });
        apply.addEventListener("click", () => {
          const amt = Number(amount.value);
          if (!Number.isFinite(amt) || amt === 0 || reason.value.trim() === "") return;
          reviewAction("/api/adjust", { command_id: Date.now(), squire: s.squire, amount: amt, reason: reason.value.trim() });
        });
        cancel.addEventListener("click", () => { editor.remove(); adj.hidden = false; });
        editor.append(amount, reason, apply, cancel);
        li.appendChild(editor);
        reason.focus();
      });
      li.append(av, info, coin, adj);
      if (pay) li.appendChild(pay);
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
      li.className = "list-row" + (m.active ? "" : " inactive");

      const info = document.createElement("span");
      info.className = "grow";
      const name = document.createElement("strong");
      name.textContent = m.display_name + " ";
      const role = document.createElement("span");
      role.className = "badge " + (m.role === "Knight" ? "badge-knight" : "badge-squire");
      role.textContent = m.role;
      info.append(name, role);
      if (!m.active) {
        const inactive = document.createElement("span");
        inactive.className = "badge badge-muted";
        inactive.textContent = "Inactive";
        inactive.style.marginLeft = ".35rem";
        info.append(inactive);
      }

      // Their colours (SQUIRE-T-0136): a swatch per tincture; the chosen one is ringed. Squires only —
      // it is the colour THEIR screens wear.
      if (m.role === "Squire") {
        const swatches = document.createElement("span");
        swatches.className = "tinctures";
        swatches.setAttribute("role", "radiogroup");
        swatches.setAttribute("aria-label", `${m.display_name}'s colour`);
        for (const t of TINCTURES) {
          const b = document.createElement("button");
          b.type = "button";
          b.className = "tincture" + (m.tincture === t ? " chosen" : "");
          b.style.setProperty("--t", TINCTURE_HEX[t]);
          b.title = t[0].toUpperCase() + t.slice(1);
          b.setAttribute("aria-label", b.title);
          b.setAttribute("aria-pressed", String(m.tincture === t));
          b.addEventListener("click", async () => {
            const r = await fetch(`/api/members/${m.user}/tincture`, {
              method: "POST",
              headers: { "content-type": "application/json" },
              body: JSON.stringify({ tincture: t }),
            });
            if (r.ok) loadMembers();
          });
          swatches.appendChild(b);
        }
        info.append(swatches);
      }

      // Rename in place (SQUIRE-T-0120/0126) — keeps the member's id, role, and pairing.
      const renameBtn = document.createElement("button");
      renameBtn.className = "btn-sm btn-ghost";
      renameBtn.textContent = "Rename";
      renameBtn.addEventListener("click", async () => {
        const next = prompt(`New name for ${m.display_name}:`, m.display_name);
        if (next == null) return;
        const name = next.trim();
        if (!name || name === m.display_name) return;
        const r = await fetch(`/api/members/${m.user}/name`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ display_name: name }),
        });
        if (r.ok) {
          loadMembers();
          loadPairMembers();
        }
      });

      const btn = document.createElement("button");
      btn.className = "btn-sm btn-ghost";
      btn.textContent = m.active ? "Deactivate" : "Reactivate";
      btn.addEventListener("click", async () => {
        await fetch(`/api/members/${m.user}/active`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ active: !m.active }),
        });
        loadMembers();
        loadPairMembers();
      });

      li.append(info, renameBtn, btn);
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
      memberTok.className = "notice notice-success";
      memberTok.innerHTML = "";
      const head = document.createElement("span");
      head.textContent = `Added ${fd.get("display_name")}. Pairing token (use it to provision their device):`;
      const tok = document.createElement("code");
      tok.className = "tok";
      tok.textContent = added.token;
      memberTok.append(head, tok);
      memberTok.hidden = false;
      memberForm.reset();
      loadMembers();
      // A new member is immediately pairable — refresh the Pair tab's member dropdown (T-0071).
      loadPairMembers();
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
      const accentCls = row.item ? (row.out_of_stock ? "accent-blue" : "accent-gold") : "accent-epic";
      const li = el("li", "card-row " + accentCls + (obj.active ? "" : " is-archived"));
      if (row.item) {
        li.appendChild(medallion(rewardEmoji(obj)));
        const meta = [];
        if (obj.gate != null) meta.push("needs " + gateLabel(obj.gate));
        if (row.out_of_stock) meta.push("out of stock");
        if (!obj.active) meta.push("archived");
        li.appendChild(cardBody(obj.name, meta.length ? meta : ["reward"]));
        li.appendChild(coinPill(obj.cost));
      } else {
        li.appendChild(medallion("🛡"));
        const meta = [criterionSummary(obj.criterion)];
        if (!obj.active) meta.push("archived");
        li.appendChild(cardBody(obj.name, meta));
        const c = el("span", "coin");
        c.innerHTML = '<span class="disc">★</span>';
        c.appendChild(el("span", "amt", "+" + obj.bonus_points));
        li.appendChild(c);
      }
      if (obj.active) {
        // Edit-in-place for rewards + achievements (SQUIRE-T-0120/0126).
        const editBtn = el("button", "edit-row", "Edit");
        editBtn.addEventListener("click", () => (row.item ? fillItemForm(obj) : fillAchForm(obj)));
        li.appendChild(editBtn);
        const btn = el("button", "archive", "Archive");
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

  // ── New-phone install QR (T-0088 / ADR A-0012) ───────────────────────────────
  // Show a QR that downloads + installs the current APK from the LAN api. Degrades to a note
  // when no build is published yet.
  async function loadInstallQr() {
    const card = document.getElementById("install-card");
    const none = document.getElementById("install-none");
    if (!card || !none) return;
    try {
      const res = await fetch("/api/app/install");
      if (!res.ok) throw new Error("install endpoint");
      const a = await res.json();
      if (!a.available) {
        card.hidden = true;
        none.hidden = false;
        return;
      }
      document.getElementById("install-qr").innerHTML = a.qr_svg || "";
      document.getElementById("install-meta").textContent =
        `Squire v${a.version_name} · ${a.url}`;
      card.hidden = false;
      none.hidden = true;
    } catch {
      card.hidden = true;
      none.hidden = false;
    }
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
  // ── Edit-in-place (SQUIRE-T-0120) ────────────────────────────────────────────
  // Authoring is an upsert: re-POSTing a definition with an EXISTING id edits it in place (the engine
  // replaces the row, keeping its id + created audit). Each catalog form can enter "edit mode" —
  // prefilled, submit relabelled "Save changes", with a Cancel — and reuses the stashed id on submit
  // instead of minting a fresh one. `null` id ⇒ a normal create.
  let questEdit = null;
  let itemEdit = null;
  let achEdit = null;
  function makeEditable(form) {
    const submit = form.querySelector('button[type="submit"]');
    const createLabel = submit ? submit.textContent : "Save";
    const cancel = el("button", "cancel-edit", "Cancel");
    cancel.type = "button";
    cancel.hidden = true;
    if (submit) submit.after(cancel);
    cancel.addEventListener("click", () => form.reset());
    const state = { id: null };
    form.addEventListener("reset", () => {
      state.id = null;
      cancel.hidden = true;
      if (submit) submit.textContent = createLabel;
    });
    state.enter = (id) => {
      state.id = id;
      cancel.hidden = false;
      if (submit) submit.textContent = "Save changes";
      form.scrollIntoView({ behavior: "smooth", block: "center" });
    };
    return state;
  }

  const questForm = document.getElementById("quest-form");
  const questErr = document.getElementById("quest-error");

  // Prefill the quest form from a raw quest object and enter edit mode (SQUIRE-T-0120). The Keep's
  // GET /api/quests returns the full domain quest, so every field round-trips.
  function fillQuestForm(q) {
    openCompose(document.getElementById("quest-form").closest("details.compose"));
    if (!questForm) return;
    const set = (name, val) => { const e = questForm.querySelector(`[name="${name}"]`); if (e) e.value = val; };
    set("title", q.title || "");
    set("reward", q.reward);
    set("cash", Number(q.cash) || 0);
    set("category", q.category || "");
    set("completion", q.completion || "EachAssignee");
    const cad = q.cadence || {};
    const cadSel = document.getElementById("quest-cadence");
    if (cad.Recurring === "Daily") {
      cadSel.value = "Daily";
    } else if (cad.Recurring && cad.Recurring.Weekly) {
      cadSel.value = "Weekly";
      const days = cad.Recurring.Weekly.days || [];
      for (const cb of questForm.querySelectorAll('input[name="wd"]')) cb.checked = days.includes(cb.value);
    } else if (cad.OneOff) {
      cadSel.value = "OneOff";
      set("due", cad.OneOff.due != null ? fromDomainDate(cad.OneOff.due) : "");
    }
    const some = !!(q.assignment && q.assignment.Squires);
    for (const r of questForm.querySelectorAll('input[name="assign"]')) r.checked = r.value === (some ? "some" : "all");
    const ids = some ? q.assignment.Squires.map(String) : [];
    for (const cb of questForm.querySelectorAll('input[name="squire"]')) cb.checked = ids.includes(cb.value);
    const auto = questForm.querySelector('[name="auto_approve"]'); if (auto) auto.checked = !!q.auto_approve;
    const rep = questForm.querySelector('[name="repeat_day"]'); if (rep) rep.checked = !!q.repeatable_within_day;
    syncQuestFields();
    questEdit.enter(q.id);
  }
  // Inverse of toDomainDate: domain day-count → yyyy-mm-dd (UTC).
  function fromDomainDate(d) {
    return new Date((d - 3) * 86400000).toISOString().slice(0, 10);
  }

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
      // The quests page lists what a squire can work towards right now — archived
      // quests stay in the store (and in achievement scopes) but are not shown here.
      if (!q.active) continue;
      const accent = q.auto_approve ? "accent-green" : "accent-royal";
      const li = el("li", "card-row " + accent);
      li.appendChild(medallion(questEmoji(q)));
      li.appendChild(cardBody(q.title, [questSummary(q)]));
      if (q.auto_approve) li.appendChild(el("span", "chip chip-auto", "Auto"));
      if (Number(q.cash) > 0) li.appendChild(el("span", "chip", `$${Number(q.cash)}`));
      li.appendChild(coinPill(q.reward));
      const editBtn = el("button", "edit-row", "Edit");
      editBtn.addEventListener("click", () => fillQuestForm(q));
      li.appendChild(editBtn);
      const btn = el("button", "archive", "Archive");
      btn.addEventListener("click", async () => {
        await fetch(`/api/quests/${q.id}/archive`, { method: "POST" });
        loadQuests();
        loadAchScopeQuests();
      });
      li.appendChild(btn);
      ul.appendChild(li);
    }
  }

  if (questForm) {
    questEdit = makeEditable(questForm);
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
        // Reuse the existing id when editing (upsert); otherwise mint a fresh one. QuestId is a u128
        // deserialized from a JSON number; Date.now() is well under 2^53 (SQUIRE-T-0120).
        id: questEdit.id ?? Date.now(),
        title: fd.get("title"),
        description: null,
        category: category || null,
        reward: Number(fd.get("reward")),
        cash: Number(fd.get("cash")) || 0,
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
      // A new quest must immediately appear in the achievement composer's Quest-scope dropdown,
      // even without switching tabs (SQUIRE-T-0071).
      loadAchScopeQuests();
    });
  }

  // ── Starter quest library (T-0063) ───────────────────────────────────────────
  // A shipped catalog (assets/library.json) of common chores the parent can one-tap import for
  // all squires, then edit/archive. Rendered once, grouped by category.
  let idSeq = 0;
  function freshId() {
    // Monotonic id so rapid imports don't collide on the same millisecond. Stays < 2^53.
    return Date.now() * 1000 + (idSeq++ % 1000);
  }

  let libraryLoaded = false;
  async function loadLibrary() {
    if (libraryLoaded) return;
    const res = await fetch("/static/library.json");
    if (!res.ok) return;
    const data = await res.json();
    const quests = (data && data.quests) || [];
    const container = document.getElementById("library-list");
    if (!container) return;
    container.innerHTML = "";
    libraryLoaded = true;

    const cats = [];
    const byCat = {};
    for (const q of quests) {
      if (!byCat[q.category]) { byCat[q.category] = []; cats.push(q.category); }
      byCat[q.category].push(q);
    }
    for (const cat of cats) {
      const h = document.createElement("h3");
      h.textContent = cat;
      container.appendChild(h);
      const ul = document.createElement("ul");
      for (const q of byCat[cat]) {
        const cadenceLabel = q.cadence === "weekly" ? (q.days || []).join("/") : "Daily";
        const li = document.createElement("li");
        const strong = document.createElement("strong");
        strong.textContent = q.title;
        li.append(strong, document.createTextNode(` — ${q.reward} ★ · ${cadenceLabel} `));
        const btn = document.createElement("button");
        btn.textContent = "Import";
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          const cadence = q.cadence === "weekly"
            ? { Recurring: { Weekly: { days: q.days || [] } } }
            : { Recurring: "Daily" };
          const quest = {
            id: freshId(),
            title: q.title,
            description: null,
            category: q.category,
            reward: Number(q.reward),
            cadence,
            assignment: "AllSquires",
            completion: "EachAssignee",
            auto_approve: false,
            repeatable_within_day: false,
            active: true,
            icon: null,
          };
          const r = await fetch("/api/quests", {
            method: "POST",
            headers: { "content-type": "application/json" },
            body: JSON.stringify(quest),
          });
          if (r.ok) {
            btn.textContent = "Imported ✓";
            loadQuests();
            loadAchScopeQuests();
          } else {
            btn.disabled = false;
            const e = document.getElementById("library-error");
            e.textContent = "Could not import that quest.";
            e.hidden = false;
          }
        });
        li.appendChild(btn);
        ul.appendChild(li);
      }
      container.appendChild(ul);
    }
  }

  // ── Item create (T-0027) ─────────────────────────────────────────────────────
  const itemForm = document.getElementById("item-form");
  const itemErr = document.getElementById("item-error");

  // Prefill the reward form from a raw item and enter edit mode (SQUIRE-T-0120).
  function fillItemForm(it) {
    openCompose(document.getElementById("item-form").closest("details.compose"));
    if (!itemForm) return;
    const set = (name, val) => { const e = itemForm.querySelector(`[name="${name}"]`); if (e) e.value = val; };
    set("name", it.name || "");
    set("description", it.description || "");
    set("cost", it.cost);
    set("availability", it.availability || "Repeatable");
    const gate = document.getElementById("item-gate");
    if (gate) gate.value = it.gate != null ? String(it.gate) : "";
    itemEdit.enter(it.id);
  }

  if (itemForm) {
    itemEdit = makeEditable(itemForm);
    itemForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      itemErr.hidden = true;
      const fd = new FormData(itemForm);
      const name = (fd.get("name") || "").trim();
      const cost = Number(fd.get("cost"));
      // Form-level guards (the engine validates the gate, not the cost/name).
      if (!name) { itemErr.textContent = "Add a name."; itemErr.hidden = false; return; }
      if (!Number.isFinite(cost) || cost < 1) { itemErr.textContent = "Cost must be at least 1 point."; itemErr.hidden = false; return; }
      const desc = (fd.get("description") || "").trim();
      const gateRaw = fd.get("gate");
      const item = {
        id: itemEdit.id ?? Date.now(), // reuse id when editing (upsert) — SQUIRE-T-0120
        name,
        description: desc || null,
        cost,
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
        // 404 = the chosen gate achievement no longer exists (validate_item).
        itemErr.textContent = res.status === 404 ? "That gate achievement no longer exists — pick another." : "Could not save reward.";
        itemErr.hidden = false;
        return;
      }
      itemForm.reset();
      loadCatalog("items", "item-list");
    });
  }

  // ── Starter reward library (T-0073) ──────────────────────────────────────────
  // Common household rewards (assets/rewards-library.json), one-tap import — mirrors the quest +
  // achievement libraries. Reuses freshId() so rapid imports don't collide.
  let rewardsLibraryLoaded = false;
  async function loadRewardsLibrary() {
    if (rewardsLibraryLoaded) return;
    const res = await fetch("/static/rewards-library.json");
    if (!res.ok) return;
    const data = await res.json();
    const list = (data && data.rewards) || [];
    const container = document.getElementById("rewards-library-list");
    if (!container) return;
    container.innerHTML = "";
    rewardsLibraryLoaded = true;
    const ul = document.createElement("ul");
    for (const r of list) {
      const li = document.createElement("li");
      const strong = document.createElement("strong");
      strong.textContent = r.name;
      li.append(strong, document.createTextNode(` — ${rewardLibSummary(r)} `));
      const btn = document.createElement("button");
      btn.textContent = "Import";
      btn.addEventListener("click", async () => {
        btn.disabled = true;
        const resp = await fetch("/api/items", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(rewardLibToWire(r)),
        });
        if (resp.ok) {
          btn.textContent = "Imported ✓";
          loadCatalog("items", "item-list");
        } else {
          btn.disabled = false;
          const e = document.getElementById("rewards-library-error");
          e.textContent = "Could not import that reward.";
          e.hidden = false;
        }
      });
      li.appendChild(btn);
      ul.appendChild(li);
    }
    container.appendChild(ul);
  }

  function rewardLibToWire(r) {
    return {
      id: freshId(),
      name: r.name,
      description: r.description || null,
      cost: Number(r.cost),
      gate: null,
      availability: r.availability || "Repeatable",
      active: true,
      icon: null,
    };
  }
  function rewardLibSummary(r) {
    const avail = r.availability === "Once" ? "once" : "repeatable";
    return `${r.cost} pts · ${avail}`;
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
    if (scope === "Category") return { Category: (fd.get("scope_category") || "").trim() };
    return "Any";
  }

  // Prefill the achievement form from a raw achievement and enter edit mode (SQUIRE-T-0120/0126),
  // round-tripping the criterion (PointsEarned | TotalCompletions | Streak) and its scope.
  function fillAchForm(a) {
    openCompose(document.getElementById("achievement-form").closest("details.compose"));
    if (!achForm) return;
    const set = (name, val) => { const e = achForm.querySelector(`[name="${name}"]`); if (e != null) e.value = val; };
    set("name", a.name || "");
    set("bonus", a.bonus_points);
    const critSel = document.getElementById("ach-criterion");
    const scopeSel = document.getElementById("ach-scope");
    const setScope = (scope) => {
      if (scope && scope.Quest != null) { scopeSel.value = "Quest"; set("scope_quest", String(scope.Quest)); }
      else if (scope && scope.Category != null) { scopeSel.value = "Category"; set("scope_category", scope.Category); }
      else { scopeSel.value = "Any"; }
    };
    const c = a.criterion || {};
    if (c.PointsEarned) {
      critSel.value = "PointsEarned";
      set("total", c.PointsEarned.total);
    } else if (c.TotalCompletions) {
      critSel.value = "TotalCompletions";
      set("count", c.TotalCompletions.count);
      setScope(c.TotalCompletions.scope);
    } else if (c.Streak) {
      critSel.value = "Streak";
      set("length", c.Streak.length);
      set("basis", c.Streak.basis);
      setScope(c.Streak.scope);
    }
    syncAchFields();
    achEdit.enter(a.id);
  }

  if (achForm) {
    achEdit = makeEditable(achForm);
    document.getElementById("ach-criterion").addEventListener("change", syncAchFields);
    document.getElementById("ach-scope").addEventListener("change", syncAchFields);
    syncAchFields();

    achForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      achErr.hidden = true;
      const fd = new FormData(achForm);
      const kind = fd.get("criterion");
      // A Category scope must be non-blank (the domain rejects an empty one — SQUIRE-T-0071).
      if (kind !== "PointsEarned" && fd.get("scope") === "Category" && !(fd.get("scope_category") || "").trim()) {
        achErr.textContent = "Enter a category for the scope.";
        achErr.hidden = false;
        return;
      }
      let criterion;
      if (kind === "PointsEarned") {
        criterion = { PointsEarned: { total: Number(fd.get("total")) } };
      } else if (kind === "TotalCompletions") {
        criterion = { TotalCompletions: { scope: buildScope(fd), count: Number(fd.get("count")) } };
      } else {
        criterion = { Streak: { scope: buildScope(fd), length: Number(fd.get("length")), basis: fd.get("basis") } };
      }
      const achievement = {
        id: achEdit.id ?? Date.now(), // reuse id when editing (upsert) — SQUIRE-T-0120/0126
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

  // ── Starter achievement library (T-0071) ─────────────────────────────────────
  // Common milestones scoped to the quest-library categories (plus Any/points), one-tap import —
  // mirrors the quest library. Reuses freshId() from the quest section.
  let achLibraryLoaded = false;
  async function loadAchLibrary() {
    if (achLibraryLoaded) return;
    const res = await fetch("/static/achievements-library.json");
    if (!res.ok) return;
    const data = await res.json();
    const list = (data && data.achievements) || [];
    const container = document.getElementById("ach-library-list");
    if (!container) return;
    container.innerHTML = "";
    achLibraryLoaded = true;
    const ul = document.createElement("ul");
    for (const a of list) {
      const li = document.createElement("li");
      const strong = document.createElement("strong");
      strong.textContent = a.name;
      li.append(strong, document.createTextNode(` — ${achLibSummary(a)} · +${a.bonus} ★ `));
      const btn = document.createElement("button");
      btn.textContent = "Import";
      btn.addEventListener("click", async () => {
        btn.disabled = true;
        const r = await fetch("/api/achievements", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify(achLibToWire(a)),
        });
        if (r.ok) {
          btn.textContent = "Imported ✓";
          loadCatalog("achievements", "achievement-list");
        } else {
          btn.disabled = false;
          const e = document.getElementById("ach-library-error");
          e.textContent = "Could not import that achievement.";
          e.hidden = false;
        }
      });
      li.appendChild(btn);
      ul.appendChild(li);
    }
    container.appendChild(ul);
  }

  function achLibScope(a) {
    return a.scope === "category" ? { Category: a.category } : "Any";
  }
  function achLibToWire(a) {
    let criterion;
    if (a.criterion === "points") criterion = { PointsEarned: { total: a.total } };
    else if (a.criterion === "total") criterion = { TotalCompletions: { scope: achLibScope(a), count: a.count } };
    else criterion = { Streak: { scope: achLibScope(a), length: a.length, basis: a.basis } };
    return { id: freshId(), name: a.name, description: null, criterion, bonus_points: a.bonus, active: true };
  }
  function achLibSummary(a) {
    const scope = a.scope === "category" ? a.category : "any";
    if (a.criterion === "points") return `${a.total} points`;
    if (a.criterion === "total") return `${a.count} completions · ${scope}`;
    return `${a.length}-day streak · ${scope}`;
  }

  // ── Event-log inspector (T-0030) ─────────────────────────────────────────────
  const logForm = document.getElementById("log-form");
  if (logForm) {
    logForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const fd = new FormData(logForm);
      const res = await fetch(`/api/log/${fd.get("scope")}/${fd.get("id")}`);
      const out = document.getElementById("log-output");
      const hint = document.getElementById("log-hint");
      out.textContent = res.ok ? JSON.stringify(await res.json(), null, 2) : `Error ${res.status}`;
      out.hidden = false;
      if (hint) hint.hidden = true;
    });
  }

  // ── Household settings — timezone (T-0068) ───────────────────────────────────
  // Common zones for the picker; any IANA zone can still be typed in the custom field.
  const COMMON_ZONES = [
    "America/New_York", "America/Detroit", "America/Chicago", "America/Denver",
    "America/Phoenix", "America/Los_Angeles", "America/Anchorage", "Pacific/Honolulu",
    "America/Toronto", "Europe/London", "Europe/Paris", "UTC",
  ];

  async function loadSettings() {
    const res = await fetch("/api/config");
    if (!res.ok) return;
    const cfg = await res.json();
    const sel = document.getElementById("settings-tz");
    if (!sel) return;
    // Build the option list, including the current zone if it's not already one of the common ones.
    const zones = COMMON_ZONES.includes(cfg.timezone) ? COMMON_ZONES : [cfg.timezone, ...COMMON_ZONES];
    sel.innerHTML = "";
    for (const z of zones) {
      const opt = document.createElement("option");
      opt.value = z;
      opt.textContent = z;
      if (z === cfg.timezone) opt.selected = true;
      sel.appendChild(opt);
    }
    document.getElementById("settings-tz-custom").value = "";
    // Quiet hours (SQUIRE-T-0138): stored as minutes since midnight, shown as a clock time.
    document.getElementById("settings-wake-from").value = hhmm(cfg.notify_wake_from);
    document.getElementById("settings-wake-to").value = hhmm(cfg.notify_wake_to);
  }

  /** minutes-since-midnight → "07:00" for an <input type=time>. */
  function hhmm(minutes) {
    const m = Number.isFinite(minutes) ? minutes : 0;
    return String(Math.floor(m / 60)).padStart(2, "0") + ":" + String(m % 60).padStart(2, "0");
  }
  /** "07:00" → minutes since midnight, or null when the field is empty. */
  function minutesOfDay(value) {
    const m = /^(\d{1,2}):(\d{2})$/.exec((value || "").trim());
    return m ? Number(m[1]) * 60 + Number(m[2]) : null;
  }

  const settingsForm = document.getElementById("settings-form");
  if (settingsForm) {
    settingsForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const status = document.getElementById("settings-status");
      const error = document.getElementById("settings-error");
      status.hidden = true;
      error.hidden = true;
      const custom = document.getElementById("settings-tz-custom").value.trim();
      const timezone = custom || document.getElementById("settings-tz").value;
      const wakeFrom = minutesOfDay(document.getElementById("settings-wake-from").value);
      const wakeTo = minutesOfDay(document.getElementById("settings-wake-to").value);
      if (wakeFrom !== null && wakeTo !== null && wakeFrom >= wakeTo) {
        error.textContent = "Quiet-until must come before quiet-after.";
        error.hidden = false;
        return;
      }
      const res = await fetch("/api/config", {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ timezone, notify_wake_from: wakeFrom, notify_wake_to: wakeTo }),
      });
      if (res.ok) {
        const cfg = await res.json();
        status.textContent = `Saved — daily quests now reset at midnight in ${cfg.timezone}.`;
        status.hidden = false;
        loadSettings();
      } else {
        error.textContent = res.status === 400 ? `"${timezone}" isn't a valid IANA timezone.` : "Could not save settings.";
        error.hidden = false;
      }
    });
  }

  // ── Change my secret (SQUIRE-T-0131) ─────────────────────────────────────────────────────────
  const secretForm = document.getElementById("secret-form");
  if (secretForm) {
    secretForm.addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const status = document.getElementById("secret-status");
      const error = document.getElementById("secret-error");
      status.hidden = true;
      error.hidden = true;
      const fail = (msg) => { error.textContent = msg; error.hidden = false; };
      const current_secret = document.getElementById("secret-current").value;
      const new_secret = document.getElementById("secret-new").value;
      if (new_secret !== document.getElementById("secret-confirm").value) return fail("The two new secrets don't match.");
      if (new_secret.trim().length < 8) return fail("The new secret needs at least 8 characters.");
      const res = await fetch("/api/me/secret", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ current_secret, new_secret }),
      });
      if (res.ok) {
        secretForm.reset();
        status.textContent = "Secret changed. Use the new one next time you sign in.";
        status.hidden = false;
      } else {
        fail(
          res.status === 401 ? "That isn't your current secret."
          : res.status === 429 ? "Too many failed attempts — this account is locked for a while. Try again later."
          : res.status === 400 ? "The new secret needs at least 8 characters."
          : "Could not change the secret."
        );
      }
    });
  }

  // ── Hazards (SQUIRE-T-0096): the named-penalty catalog (shared config) + quick-apply ──────────
  let hazards = [];
  async function loadHazards() {
    const res = await fetch("/api/hazards");
    hazards = res.ok ? await res.json() : [];
    if (Object.keys(squiresById).length === 0) await loadQuestSquires();
    renderHazards();
  }
  async function saveHazards(next) {
    const res = await fetch("/api/hazards", {
      method: "PUT",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(next),
    });
    if (res.ok) { hazards = await res.json(); renderHazards(); }
  }
  function renderHazards() {
    const box = document.getElementById("hazard-list");
    if (!box) return;
    box.innerHTML = "";
    if (hazards.length === 0) { box.appendChild(el("p", "sub", "No hazards yet.")); return; }
    for (const h of hazards) {
      const card = el("div", "list-row");
      const title = el("div", "card-title");
      title.appendChild(el("strong", null, h.name));
      title.appendChild(el("span", "sub", `  ·  −${h.penalty} coins`));
      card.appendChild(title);

      const applyRow = el("div", "sub");
      applyRow.appendChild(el("span", null, "Apply to: "));
      const ids = Object.keys(squiresById);
      if (ids.length === 0) {
        applyRow.appendChild(el("span", null, "(no squires)"));
      } else {
        for (const id of ids) {
          const b = el("button", null, squiresById[id]);
          b.type = "button";
          b.addEventListener("click", async () => {
            await fetch("/api/adjust", {
              method: "POST",
              headers: { "content-type": "application/json" },
              body: JSON.stringify({ command_id: Date.now(), squire: Number(id), amount: -Number(h.penalty), reason: h.name }),
            });
            b.textContent = "✓ " + squiresById[id];
          });
          applyRow.appendChild(b);
          applyRow.appendChild(el("span", null, " "));
        }
      }
      card.appendChild(applyRow);

      const rm = el("button", null, "Remove");
      rm.type = "button";
      rm.addEventListener("click", () => saveHazards(hazards.filter((x) => x.name !== h.name)));
      card.appendChild(rm);
      box.appendChild(card);
    }
  }
  {
    const form = document.getElementById("hazard-form");
    if (form) {
      form.addEventListener("submit", async (ev) => {
        ev.preventDefault();
        const fd = new FormData(form);
        const name = (fd.get("name") || "").toString().trim();
        const penalty = Number(fd.get("penalty"));
        const err = document.getElementById("hazard-error");
        if (!name || !(penalty >= 1)) {
          err.textContent = "Name and coins (at least 1) are required.";
          err.hidden = false;
          return;
        }
        err.hidden = true;
        await saveHazards([...hazards, { name, penalty, icon: null }]);
        form.reset();
      });
    }
  }
})();
