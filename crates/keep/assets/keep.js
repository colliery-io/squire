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
  });
})();
