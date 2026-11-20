// Drives the Super Keyboard browser preview like a user, to take the shots in
// this folder: theme, profile, the VI switch and taps on on-screen keys by
// their labels. Replace __PLAN__ with a plan, run it in the page, then take a
// screenshot. The plans for the shots:
//   desktop.png  1280x760 @2x  {"theme":"Tokyo Night","profile":"coding","keys":"fn main() {\n\tkeyboard.run();\n}","after":["ctrl"]}
//   phone.png     390x820 @3x  {"theme":"Catppuccin Latte","profile":"coding","vi":true,"keys":"Xin chaof, Vieejt Nam"}
//   vntab.png    1280x900 @2x  {"theme":"Tokyo Night","vi":true,"settingsTab":"Việt"}
// Serve the preview with: dx serve -p super-keyboard --web --no-default-features --features web
(async (plan) => {
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const byText = (sel, text) => [...document.querySelectorAll(sel)].find(b => b.textContent.trim() === text);
  const click = async (el, what) => { if (!el) throw new Error("not found: " + what); el.click(); await sleep(180); };
  for (let i = 0; i < 60 && !document.querySelector(".key"); i++) await sleep(250);
  if (plan.theme) {
    await click(byText(".bar-button", "settings"), "settings");
    await click([...document.querySelectorAll(".tab")].find(t => t.textContent.includes("Themes")), "Themes tab");
    await click(byText(".bar-button", "one theme"), "one theme");
    await click(document.querySelector(`[aria-label="${plan.theme}"]`), plan.theme);
    await click(byText(".bar-button", "done"), "done");
  }
  if (plan.profile) {
    const s = document.querySelector(".profile-picker");
    s.value = plan.profile; s.dispatchEvent(new Event("change", { bubbles: true })); await sleep(250);
  }
  if (plan.vi) await click(document.querySelector(".lang-toggle"), "VI");
  const tap = async el => {
    const o = { bubbles: true, pointerId: 1, clientX: 10, isPrimary: true };
    el.dispatchEvent(new PointerEvent("pointerdown", o)); await sleep(25);
    el.closest(".keyboard").dispatchEvent(new PointerEvent("pointerup", o)); await sleep(45);
  };
  const names = { " ": "space", "\n": "⏎", "\t": "tab", "\b": "⌫" };
  const key = label => [...document.querySelectorAll(".key")].find(
    b => (b.querySelector(".label")?.textContent.trim() || "").toLowerCase() === label.toLowerCase());
  const missing = [];
  for (const ch of plan.keys || "") {
    if (/[A-Z]/.test(ch)) await tap(key("⇧"));
    const el = key(names[ch] || ch);
    if (el) await tap(el); else missing.push(ch);
  }
  for (const label of plan.after || []) await tap(key(label));
  if (plan.settingsTab) {
    await click(byText(".bar-button", "settings"), "settings");
    await click([...document.querySelectorAll(".tab")].find(t => t.textContent.includes(plan.settingsTab)), plan.settingsTab);
  }
  await sleep(400);
  return JSON.stringify({ missing, text: document.querySelector(".preview .text")?.innerText ?? null });
})(__PLAN__)
