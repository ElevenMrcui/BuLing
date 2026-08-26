// 不令 OPC · 深浅色三态（浅色 / 深色 / 跟随系统）。
//
// Tailwind 的 `darkMode: ["class"]` 只认 <html> 上有没有 `.dark`——它自己不
// 认"跟随系统"这个第三态。所以"系统"选项在这里用 JS 解出当前 OS 是否是
// 深色（`matchMedia`），跟"深色"选项一样切成同一个 `.dark` class；区别只是
// "系统"会继续监听 OS 主题变化，"深色"/"浅色"是用户的显式覆盖，不跟着变。
// 这跟 next-themes 之类库的常见做法一致。

import { useCallback, useEffect, useState } from "react";

export type ThemeChoice = "light" | "dark" | "system";

const STORAGE_KEY = "opc-theme";

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function applyResolvedTheme(choice: ThemeChoice) {
  const dark = choice === "dark" || (choice === "system" && systemPrefersDark());
  document.documentElement.classList.toggle("dark", dark);
}

function readStoredTheme(): ThemeChoice {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    // localStorage 不可用（隐私模式等）——退回默认值，不影响渲染。
  }
  return "system";
}

export function useTheme() {
  const [choice, setChoiceState] = useState<ThemeChoice>(() => readStoredTheme());

  useEffect(() => {
    applyResolvedTheme(choice);
    if (choice !== "system") return;
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => applyResolvedTheme("system");
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, [choice]);

  const setChoice = useCallback((next: ThemeChoice) => {
    setChoiceState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // 存不进去就算了，本次会话内的切换仍然生效。
    }
  }, []);

  return { choice, setChoice };
}
