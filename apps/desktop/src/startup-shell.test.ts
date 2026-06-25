import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const currentDir = dirname(fileURLToPath(import.meta.url));
const indexHtml = readFileSync(join(currentDir, "..", "index.html"), "utf8");

function inlineStyleBlock() {
  const match = indexHtml.match(/<style>(?<css>[\s\S]*?)<\/style>/);
  return match?.groups?.css ?? "";
}

test("index html renders a branded startup shell before React loads", () => {
  expect(indexHtml).toContain('id="preload-splash"');
  expect(indexHtml).toContain("AI Session Migrator");
  expect(indexHtml).toContain("Codex 会话迁移助手");
  expect(indexHtml).toContain('id="root"');
  expect(indexHtml.indexOf('id="preload-splash"')).toBeLessThan(indexHtml.indexOf('id="root"'));
});

test("startup shell uses the final splash palette instead of a blank white page", () => {
  const css = inlineStyleBlock();

  expect(css).toContain("#fbf8ff");
  expect(css).toContain("#a53c05");
  expect(css).toContain("#6d3bd7");
  expect(css).toContain("position: fixed");
  expect(css).toContain("inset: 0");
});
