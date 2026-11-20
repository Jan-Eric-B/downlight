// The settings flyout (window design ticket 08): the reader's Theme, Backdrop, Accent color and
// Edit with, under the title bar's gear, above the document rather than over the whole window, so
// each choice shows on the document at once. A choice is a `SettingChange`, which the app keeps
// in the settings and sends every window; the flyout shows the settings as every window hears
// them (`whenSettingsChange`), so a change from another window shows here too.
//
// The gear is the `settings` command's button (commands.ts), which only toggles the flyout. Its
// open state, `aria-expanded`, is the flyout's, set here on the button found by its command, so
// the command table doesn't refresh it and neither module imports the other's internals (the
// review of window design tickets 04 to 07).
import {
  ACCENT_COLORS,
  askApp,
  BACKDROPS,
  pageFlags,
  THEMES,
  type AccentColor,
  type Backdrop,
  type EditorChoices,
  type PageSettings,
  type Theme,
} from "./app";
import { chooseEditor } from "./edit";
import { showingWhy } from "./notice";
import { changeSetting, settingsNow, whenSettingsChange } from "./settings";

const THEME_LABELS: Record<Theme, string> = {
  system: "System",
  light: "Light",
  dark: "Dark",
};

const BACKDROP_LABELS: Record<Backdrop, string> = {
  mica: "Mica",
  micaAlt: "Mica Alt",
  acrylic: "Acrylic",
  none: "None",
};

const ACCENT_COLOR_LABELS: Record<AccentColor, string> = {
  amber: "Amber",
  windows: "Windows",
};

/** Why Backdrop is disabled where the OS shows no backdrop, as on Windows 10. */
const NO_BACKDROP_REASON = "A backdrop needs Windows 11.";

const FLYOUT_ID = "settings-flyout";

/** The gear, as the title bar draws it, or null without a title bar (other systems). */
function gear(): HTMLButtonElement | null {
  return document.querySelector<HTMLButtonElement>(
    'button[data-command="settings"]',
  );
}

/** A choice of a group: a radio button with its label around it. */
function radio(
  name: string,
  value: string,
  label: string,
): { label: HTMLLabelElement; input: HTMLInputElement } {
  const input = document.createElement("input");
  input.type = "radio";
  input.name = name;
  input.value = value;
  const element = document.createElement("label");
  element.append(input, label);
  return { label: element, input };
}

/** A group of the flyout's controls, named by its legend. */
function fieldset(legend: string): HTMLFieldSetElement {
  const group = document.createElement("fieldset");
  const name = document.createElement("legend");
  name.textContent = legend;
  group.append(name);
  return group;
}

/**
 * A group of choices, one of `values` checked at a time, that runs `pick` with the value the
 * reader checks, and `show`s the value the settings hold.
 */
function choiceGroup<V extends string>(
  legend: string,
  values: readonly V[],
  labels: Record<V, string>,
  pick: (value: V) => void,
): { element: HTMLFieldSetElement; show(value: V): void } {
  const element = fieldset(legend);
  const choices = document.createElement("div");
  choices.className = "choices";
  const name = `settings-${legend.toLowerCase().replaceAll(" ", "-")}`;
  const inputs = values.map((value) => {
    const { label, input } = radio(name, value, labels[value]);
    input.addEventListener("change", () => {
      if (input.checked) pick(value);
    });
    choices.append(label);
    return input;
  });
  element.append(choices);
  return {
    element,
    show(value) {
      for (const input of inputs) input.checked = input.value === value;
    },
  };
}

/**
 * Edit with: Default, named after the program Edit opens a document in without a chosen editor,
 * the chosen editor if Edit would open it, and Choose..., the app's file dialog. What it offers
 * comes from the app (`editor_choices`), asked for each time the flyout opens and after each
 * change here, since the editor is no setting the pages hear.
 */
function editWithGroup(): { element: HTMLFieldSetElement; refresh(): void } {
  const element = fieldset("Edit with");
  const choices = document.createElement("div");
  choices.className = "editors";
  const choose = document.createElement("button");
  choose.type = "button";
  choose.textContent = "Choose...";
  element.append(choices, choose);

  /** The latest request, so that an answer an earlier one overtook shows nothing. */
  let asked = 0;
  const refresh = () => {
    const request = ++asked;
    const answer = askApp("editor_choices");
    showingWhy(answer);
    void answer.then(
      (offered) => {
        if (request === asked) show(offered);
      },
      () => undefined,
    );
  };
  const show = ({ defaultName, editor }: EditorChoices) => {
    const name = "settings-edit-with";
    const byDefault = radio(
      name,
      "default",
      defaultName === null ? "Default" : `Default (${defaultName})`,
    );
    byDefault.input.checked = editor === null;
    byDefault.input.addEventListener("change", () => {
      void changeSetting({ setting: "clearEditor" }).then(refresh);
    });
    choices.replaceChildren(byDefault.label);
    if (editor !== null) {
      const chosen = radio(name, "editor", editor.name);
      chosen.label.title = editor.path;
      chosen.input.checked = true;
      choices.append(chosen.label);
    }
  };
  choose.addEventListener("click", () => {
    void chooseEditor().then(refresh);
  });
  return { element, refresh };
}

/**
 * The places the focus stops at with Tab, in order: each enabled button, and of each group of
 * radio buttons only its checked one, or its first while none is, as the browser's own Tab does.
 */
function tabStops(flyout: HTMLElement): HTMLElement[] {
  const stops: HTMLElement[] = [];
  const groupsSeen = new Set<string>();
  for (const control of flyout.querySelectorAll<
    HTMLInputElement | HTMLButtonElement
  >("input, button")) {
    if (control.disabled || control.closest("fieldset")?.disabled) continue;
    if (control instanceof HTMLInputElement && control.type === "radio") {
      if (groupsSeen.has(control.name)) continue;
      groupsSeen.add(control.name);
      const group = [
        ...flyout.querySelectorAll<HTMLInputElement>(
          `input[name="${control.name}"]`,
        ),
      ];
      stops.push(group.find((input) => input.checked) ?? control);
      continue;
    }
    stops.push(control);
  }
  return stops;
}

interface Flyout {
  isOpen(): boolean;
  open(): void;
  close(focusGear: boolean): void;
}

let flyout: Flyout | undefined;

/**
 * The `settings` command (Ctrl+, and the gear): opens the flyout, or closes it and returns the
 * focus to the gear. Does nothing before `startSettingsFlyout`.
 */
export function toggleSettingsFlyout(): void {
  if (!flyout) return;
  if (flyout.isOpen()) flyout.close(true);
  else flyout.open();
}

/**
 * Whether the flyout is open, for Esc's order, whose later listeners defer to the flyout (full
 * screen ticket 02). False before `startSettingsFlyout`.
 */
export function isSettingsFlyoutOpen(): boolean {
  return flyout?.isOpen() ?? false;
}

type FlyoutListener = (open: boolean) => void;

const flyoutListeners = new Set<FlyoutListener>();

/**
 * Runs `listener` with whether the flyout is open each time it opens or closes, for the title bar,
 * which stays in view in full screen while the flyout hanging from its gear is open (full screen
 * ticket 04). Returns a function that removes the listener.
 */
export function whenSettingsFlyoutToggles(
  listener: FlyoutListener,
): () => void {
  flyoutListeners.add(listener);
  return () => {
    flyoutListeners.delete(listener);
  };
}

/**
 * Draws the settings flyout, closed, and keeps it in step with the settings. After
 * `startSettings` and the title bar, whose gear it opens under. The signal removes it again, for
 * the tests.
 */
export function startSettingsFlyout(signal?: AbortSignal): void {
  const element = document.createElement("div");
  element.id = FLYOUT_ID;
  element.className = "settings-flyout";
  element.setAttribute("role", "dialog");
  element.setAttribute("aria-label", "Settings");
  element.hidden = true;

  const theme = choiceGroup("Theme", THEMES, THEME_LABELS, (value) => {
    void changeSetting({ setting: "theme", theme: value });
  });
  const backdrop = choiceGroup(
    "Backdrop",
    BACKDROPS,
    BACKDROP_LABELS,
    (value) => {
      void changeSetting({ setting: "backdrop", backdrop: value });
    },
  );
  if (!pageFlags().backdropSupported) {
    backdrop.element.disabled = true;
    backdrop.element.title = NO_BACKDROP_REASON;
  }
  const accentColor = choiceGroup(
    "Accent color",
    ACCENT_COLORS,
    ACCENT_COLOR_LABELS,
    (value) => {
      void changeSetting({ setting: "accentColor", accentColor: value });
    },
  );
  const editWith = editWithGroup();
  element.append(theme.element, backdrop.element);
  // Where the OS has no accent color of its own, Amber is the one choice left, and a group of one
  // choice is none (window design ticket 09).
  if (settingsNow().windowsAccent !== null) element.append(accentColor.element);
  element.append(editWith.element);
  document.body.append(element);

  const show = (settings: PageSettings) => {
    theme.show(settings.theme);
    backdrop.show(settings.backdrop);
    accentColor.show(settings.accentColor);
  };
  const stopFollowing = whenSettingsChange(show);

  const button = gear();
  button?.setAttribute("aria-haspopup", "dialog");
  button?.setAttribute("aria-controls", FLYOUT_ID);
  button?.setAttribute("aria-expanded", "false");

  /** Where the focus was before the flyout opened, for a window without a gear. */
  let focusBefore: Element | null = null;

  /** Under the gear, its right edge in line with the gear's. */
  const place = () => {
    const anchor = gear();
    if (!anchor) return;
    const right = Math.max(
      0,
      innerWidth - anchor.getBoundingClientRect().right,
    );
    element.style.right = `${String(right)}px`;
  };

  const opened: Flyout = {
    isOpen: () => !element.hidden,
    open() {
      focusBefore = document.activeElement;
      place();
      element.hidden = false;
      gear()?.setAttribute("aria-expanded", "true");
      for (const listener of flyoutListeners) listener(true);
      editWith.refresh();
      tabStops(element)[0]?.focus();
    },
    close(focusGear) {
      element.hidden = true;
      const anchor = gear();
      anchor?.setAttribute("aria-expanded", "false");
      if (focusGear) {
        if (anchor) anchor.focus();
        else if (focusBefore instanceof HTMLElement) focusBefore.focus();
      }
      // Once the focus is where it goes, so a title bar that keeps the focus on its gear in view
      // never slides away in between.
      for (const listener of flyoutListeners) listener(false);
    },
  };
  flyout = opened;

  // Esc closes the flyout, and only the flyout: Find's Esc listens on the window.
  element.addEventListener(
    "keydown",
    (event) => {
      if (event.ctrlKey || event.altKey || event.metaKey) return;
      if (event.key === "Escape" && !event.isComposing) {
        event.preventDefault();
        event.stopPropagation();
        opened.close(true);
        return;
      }
      if (event.key !== "Tab") return;
      // The focus stays inside while the flyout is open, wrapping at either end.
      event.preventDefault();
      const stops = tabStops(element);
      if (stops.length === 0) return;
      const at = stops.indexOf(document.activeElement as HTMLElement);
      const step = event.shiftKey ? -1 : 1;
      const next =
        at === -1
          ? event.shiftKey
            ? stops.length - 1
            : 0
          : (at + step + stops.length) % stops.length;
      stops[next]?.focus();
    },
    { signal },
  );

  // A click outside closes it and leaves the focus where the click puts it, in the document. A
  // click on the gear is the gear's: it closes the flyout itself.
  addEventListener(
    "pointerdown",
    (event) => {
      if (!opened.isOpen() || !(event.target instanceof Node)) return;
      if (element.contains(event.target)) return;
      if (gear()?.contains(event.target)) return;
      opened.close(false);
    },
    { signal, capture: true },
  );

  addEventListener(
    "resize",
    () => {
      if (opened.isOpen()) place();
    },
    { signal },
  );

  signal?.addEventListener("abort", () => {
    stopFollowing();
    element.remove();
    if (flyout === opened) flyout = undefined;
  });
}
