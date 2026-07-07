import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const currentDir = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(currentDir, "styles.css"), "utf8").replace(/\r\n?/g, "\n");

function cssRule(selector: string) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = css.match(new RegExp(`${escapedSelector}\\s*\\{(?<body>[\\s\\S]*?)\\}`));
  return match?.groups?.body ?? "";
}

test("metrics are styled as a compact status strip", () => {
  const metrics = cssRule(".metrics");
  const metricItem = cssRule(".metrics div");
  const metricValue = cssRule(".metrics strong");

  expect(metrics).toContain("gap: 12px");
  expect(metricItem).toContain("min-height: 62px");
  expect(metricItem).toContain("padding: 12px 16px");
  expect(metricValue).toContain("font-size: 24px");
  expect(metricItem).not.toContain("box-shadow: var(--shadow-card)");
});

test("confirmation dialog content scrolls without hiding the action buttons", () => {
  const dialog = cssRule(".confirm-dialog,\n.details-dialog");
  const body = cssRule(".confirm-dialog-body");

  expect(dialog).toContain("max-height: calc(100dvh - 48px)");
  expect(dialog).toContain("grid-template-rows: auto minmax(0, 1fr) auto");
  expect(dialog).toContain("overflow: hidden");
  expect(body).toContain("min-height: 0");
  expect(body).toContain("overflow-y: auto");
  expect(body).toContain("overflow-wrap: anywhere");
});

test("transcript dialog keeps long conversation records scrollable", () => {
  const dialog = cssRule(".transcript-dialog");
  const title = cssRule(".transcript-dialog-title");
  const list = cssRule(".transcript-list");
  const text = cssRule(".transcript-text");
  const omitted = cssRule(".transcript-omitted");

  expect(dialog).toContain("width: min(860px, 100%)");
  expect(title).toContain("-webkit-line-clamp: 2");
  expect(title).toContain("overflow: hidden");
  expect(title).toContain("overflow-wrap: anywhere");
  expect(list).toContain("overflow-y: auto");
  expect(list).toContain("min-height: 0");
  expect(text).toContain("white-space: pre-wrap");
  expect(text).toContain("overflow-wrap: anywhere");
  expect(omitted).toContain("background: rgba(255, 243, 223, 0.64)");
  expect(omitted).toContain("font-size: 13px");
});

test("startup splash is fixed and only animates compositor-friendly properties", () => {
  const splash = cssRule(".splash-screen");
  const stage = cssRule(".splash-stage");
  const flowNode = cssRule(".splash-flow-node");
  const line = cssRule(".splash-line");

  expect(splash).toContain("position: fixed");
  expect(splash).toContain("inset: 0");
  expect(splash).toContain("z-index: 40");
  expect(stage).toContain("will-change: transform, opacity");
  expect(flowNode).toContain("will-change: transform, opacity");
  expect(line).toContain("transform-origin: center");
  expect(stage).not.toContain("top:");
  expect(flowNode).not.toContain("left:");
});
