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
    document.getElementById("quests-panel").hidden = false;
    loadQuests();
  });

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
})();
