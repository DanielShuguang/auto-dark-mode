import { defineConfig, presetWind3, transformerVariantGroup } from "unocss";

/**
 * UnoCSS 配置。
 *
 * 颜色全部映射到 CSS 变量（见 `src/styles/base.css`）：深浅色只是换一组变量值，
 * 组件里不需要写任何 `dark:` 前缀，样式表也只保留一份。
 */
export default defineConfig({
  presets: [presetWind3()],
  transformers: [transformerVariantGroup()],
  theme: {
    colors: {
      app: "var(--bg)",
      card: "var(--card)",
      field: "var(--field)",
      fg: "var(--text)",
      muted: "var(--muted)",
      line: "var(--line)",
      accent: "var(--accent)",
      "accent-soft": "var(--accent-soft)",
      danger: "var(--danger)",
    },
  },
  shortcuts: {
    panel: "bg-card border border-line rounded-[10px] p-[14px] flex flex-col gap-3",
    row: "flex items-center gap-2 flex-wrap",
    lbl: "text-muted text-[13px]",
    unit: "text-muted text-xs mr-[6px]",
    hint: "m-0 text-xs text-muted leading-[1.5]",
    warn: "m-0 text-xs text-danger leading-[1.5]",
    check: "flex items-center gap-2 cursor-pointer",
    field: "border border-line rounded-[7px] px-2 py-[6px] bg-field min-w-0",
    "btn-ghost":
      "border border-line rounded-lg px-3.5 py-2 cursor-pointer bg-card hover:bg-app disabled:opacity-60 disabled:cursor-default",
    "btn-primary":
      "border border-accent rounded-lg px-5 py-2 cursor-pointer bg-accent text-white font-semibold hover:opacity-90 disabled:opacity-60 disabled:cursor-default",
    "stat-key": "text-muted w-16 flex-none",
  },
  safelist: ["hidden"],
});
