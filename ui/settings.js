// The Settings window: each change is written to config.toml at once
// (settings.rs, set_setting) and the config hot reload restyles the documents.

const { invoke } = window.__TAURI__.core;

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const COLORS = ["background", "text", "muted", "accent", "code_background", "border"];
const FONTS = [
  ["System", "-apple-system, system-ui, sans-serif"],
  ["New York", "ui-serif, New York, Georgia, serif"],
  ["Charter", "Charter, Georgia, serif"],
  ["Iowan Old Style", "Iowan Old Style, Georgia, serif"],
  ["Avenir Next", "Avenir Next, sans-serif"],
  ["SF Mono", "ui-monospace, SF Mono, Menlo, monospace"],
];
const OTHER = "other";

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

const title = (name) => name.charAt(0).toUpperCase() + name.slice(1);

// Relative luminance of a #rrggbb color, from 0 (black) to 1 (white).
function luminance(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const channel = parseInt(hex.slice(i, i + 2), 16) / 255;
    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

// A theme card: the theme's colors on a miniature page.
function card(side, theme, selected) {
  const label = document.createElement("label");
  label.className = "card";
  for (const key of COLORS) label.style.setProperty(`--${key}`, theme.colors[key]);
  label.innerHTML = `
    <input type="radio" name="${side}-theme" />
    <span class="page" aria-hidden="true">
      <i class="heading"></i><i class="line"></i><i class="line short"></i>
      <i class="link"></i><i class="code"></i>
    </span>
    <span class="name"></span>`;
  const radio = $("input", label);
  radio.value = theme.name;
  radio.checked = theme.name === selected;
  radio.addEventListener("change", () => set([`${side}_theme`], theme.name));
  $(".name", label).textContent = title(theme.name);
  return label;
}

// The themes of a side: those whose background suits it, and the selected
// one in any case.
function fillCards(side) {
  const selected = settings[side].name;
  const themes = settings.themes.filter(
    (theme) => (luminance(theme.colors.background) > 0.5) === (side === "light") || theme.name === selected,
  );
  const cards = $(`[data-side="${side}"] .cards`);
  // Rebuilt only when a theme changed, so that keyboard focus stays put.
  const key = JSON.stringify(themes);
  if (cards.dataset.key !== key) {
    cards.dataset.key = key;
    cards.replaceChildren(...themes.map((theme) => card(side, theme, selected)));
  }
  for (const radio of $$("input", cards)) radio.checked = radio.value === selected;
}

// Fills the form, except the control being edited.
function fill() {
  const keep = (element) => element === document.activeElement;

  for (const radio of $$('[name="appearance"]')) radio.checked = radio.value === settings.appearance;

  for (const side of ["light", "dark"]) {
    fillCards(side);
    const section = $(`[data-side="${side}"]`);
    const theme = settings[side];
    $(".theme-name", section).textContent = title(theme.name);
    options($("select", section), settings.code_themes);
    for (const input of $$("[data-key]", section)) {
      if (!keep(input)) {
        input.value = input.dataset.key === "code_theme" ? theme.code_theme : theme.colors[input.dataset.key];
      }
    }
  }

  const font = FONTS.find(([, family]) => family === settings.font_family);
  if (!keep($("#font")) && !($("#font").value === OTHER && !font)) {
    $("#font").value = font ? font[1] : OTHER;
  }
  $("#font-other-row").hidden = $("#font").value !== OTHER;
  if (!keep($("#font-other"))) $("#font-other").value = font ? "" : settings.font_family;
  $("#sample").style.fontFamily = settings.font_family;

  if (!keep($("#size"))) $("#size").value = settings.font_size;

  const width = settings.max_width.match(/^(\d+)ch$/);
  if (!keep($("#width"))) $("#width").value = width ? width[1] : 70;
  $("#width-value").value = width ? `${width[1]} characters` : settings.max_width;
}

async function load() {
  settings = await invoke("get_settings");
  fill();
}

for (const radio of $$('[name="appearance"]')) {
  radio.addEventListener("change", () => set(["appearance"], radio.value));
}

for (const section of $$(".theme")) {
  const side = section.dataset.side;
  section.append($("#theme").content.cloneNode(true));

  for (const input of $$("[data-key]", section)) {
    const write = throttled((value) => set(["themes", settings[side].name, input.dataset.key], value));
    input.addEventListener(input.type === "color" ? "input" : "change", () => write(input.value));
  }
  $(".reset", section).addEventListener("click", async () => {
    const theme = settings[side].name;
    for (const key of [...COLORS, "code_theme"]) await set(["themes", theme, key], null);
  });
}

$("#font").replaceChildren(
  ...FONTS.map(([name, family]) => new Option(name, family)),
  document.createElement("hr"),
  new Option("Other…", OTHER),
);
$("#font").addEventListener("change", (event) => {
  const other = event.target.value === OTHER;
  $("#font-other-row").hidden = !other;
  if (other) $("#font-other").focus();
  else set(["font", "family"], event.target.value);
});
$("#font-other").addEventListener("change", (event) => set(["font", "family"], event.target.value.trim() || null));

$("#size").addEventListener("change", (event) => {
  const size = Number(event.target.value);
  if (size >= 8 && size <= 48) set(["font", "size"], size);
});

const writeWidth = throttled((value) => set(["layout", "max_width"], `${value}ch`));
$("#width").addEventListener("input", (event) => {
  $("#width-value").value = `${event.target.value} characters`;
  writeWidth(event.target.value);
});

$("#show-config").addEventListener("click", () =>
  invoke("reveal_config").catch((error) => ($("#error").textContent = error)),
);

// The file changed, from this window or elsewhere: show its new values.
window.__TAURI__.event.listen("config-changed", load);

await load();
