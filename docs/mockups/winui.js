/* Windows-only presentation helpers. No native window or operating-system APIs. */
(() => {
  "use strict";
  const maximize = document.querySelector("#winui-maximize");

  maximize.addEventListener("click", () => {
    const maximized = document.body.classList.toggle("winui-maximized");
    const label = maximized ? "Ripristina dimensioni del mockup" : "Massimizza il mockup";
    maximize.setAttribute("aria-pressed", String(maximized));
    maximize.setAttribute("aria-label", label);
    maximize.title = label;
    maximize.innerHTML = maximized
      ? '<svg viewBox="0 0 12 12" aria-hidden="true"><path d="M4 3V1.5h6.5V8H9"/><rect x="1.5" y="4" width="6.5" height="6.5"/></svg>'
      : '<svg viewBox="0 0 12 12" aria-hidden="true"><rect x="1.5" y="1.5" width="9" height="9"/></svg>';
  });

  document.querySelector("#taskbar-search").addEventListener("click", () => {
    document.dispatchEvent(new Event("folderai:focus-search"));
  });
  document.querySelector("#taskbar-folderai").addEventListener("click", () => {
    document.querySelector(".workspace-tab.active").focus();
  });
})();
