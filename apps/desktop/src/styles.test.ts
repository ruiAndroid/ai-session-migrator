import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const currentDir = dirname(fileURLToPath(import.meta.url));
const css = readFileSync(join(currentDir, "styles.css"), "utf8");

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
