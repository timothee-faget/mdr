// The Settings window: each change is written to config.toml at once
// (settings.rs, set_setting) and the config hot reload restyles the documents.

const { invoke } = window.__TAURI__.core;

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => [...document.querySelectorAll(selector)];
const COLORS = ["background", "text", "muted", "accent", "code_background", "border"];

let settings = null;

async function set(path, value) {
  try {
    await invoke("set_setting", { path, value });
    $("#error").textContent = "";
  } catch (error) {
    $("#error").textContent = error;
  }
}

// Dragging a color picker or a slider fires many events: write at most every
// 120 ms, and always the last value.
function throttled(write) {
  let timer = null;
  let latest;
  return (...args) => {
    latest = args;
    if (timer) return;
    timer = setTimeout(() => {
      timer = null;
      write(...latest);
    }, 120);
  };
}

function options(select, values, labels = values) {
  if (select.options.length === values.length) return;
  select.replaceChildren(...values.map((value, i) => new Option(labels[i], value)));
}

// Fills the form, except the control being edited.
function fill() {
  const keep = (element) => element === document.activeElement;
  for (const select of $$(".theme-list")) options(select, settings.themes);
  for (const select of $$(".code-themes")) options(select, settings.code_themes);

  const values = {
    "#appearance": settings.appearance,
    "#light-theme": settings.light.name,
    "#dark-theme": settings.dark.name,
    "#font": settings.font_family,
    "#size": settings.font_size,
  };
  for (const [selector, value] of Object.entries(values)) {
    if (!keep($(selector))) $(selector).value = value;
  }

  $("#light-title").textContent = `Light: ${settings.light.name}`;
  $("#dark-title").textContent = `Dark: ${settings.dark.name}`;
  for (const input of $$("[data-side]")) {
    const theme = settings[input.dataset.side];
    if (!keep(input)) {
      input.value = input.dataset.key === "code_theme" ? theme.code_theme : theme.colors[input.dataset.key];
    }
  }

  const width = settings.max_width.match(/^(\d+)ch$/);
  if (!keep($("#width"))) $("#width").value = width ? width[1] : 70;
  $("#width-value").value = width ? `${width[1]} characters` : settings.max_width;
}

async function load() {
  settings = await invoke("get_settings");
  fill();
}

$("#appearance").addEventListener("change", (event) => set(["appearance"], event.target.value));
$("#light-theme").addEventListener("change", (event) => set(["light_theme"], event.target.value));
$("#dark-theme").addEventListener("change", (event) => set(["dark_theme"], event.target.value));

for (const input of $$("[data-side]")) {
  const write = throttled((value) => {
    const theme = settings[input.dataset.side].name;
    set(["themes", theme, input.dataset.key], value);
  });
  input.addEventListener(input.type === "color" ? "input" : "change", () => write(input.value));
}

for (const button of $$("[data-reset]")) {
  button.addEventListener("click", async () => {
    const theme = settings[button.dataset.reset].name;
    for (const key of [...COLORS, "code_theme"]) await set(["themes", theme, key], null);
  });
}

$("#font").addEventListener("change", (event) => set(["font", "family"], event.target.value.trim() || null));
$("#size").addEventListener("change", (event) => {
  const size = Number(event.target.value);
  if (size >= 8 && size <= 48) set(["font", "size"], size);
});

const writeWidth = throttled((value) => set(["layout", "max_width"], `${value}ch`));
$("#width").addEventListener("input", (event) => {
  $("#width-value").value = `${event.target.value} characters`;
  writeWidth(event.target.value);
});

$("#reveal").addEventListener("click", () =>
  invoke("reveal_config").catch((error) => ($("#error").textContent = error)),
);

// The file changed, from this window or elsewhere: show its new values.
window.__TAURI__.event.listen("config-changed", load);

await load();
