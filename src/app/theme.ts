export type Theme = "system" | "light" | "dark";

export function getTheme(): Theme {
  const attr = document.documentElement.getAttribute("data-theme");
  if (attr === "light") return "light";
  if (attr === "dark") return "dark";
  return "system";
}

export function setTheme(theme: Theme): void {
  if (theme === "system") {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", theme);
  }
}
