import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, test } from "vitest";
import App from "./App";

test("renders a beginner-friendly migration workflow", () => {
  render(<App />);

  expect(screen.getByRole("heading", { name: "会话迁移助手" })).toBeInTheDocument();
  expect(screen.getByLabelText("从哪个 provider 迁出")).toHaveValue("");
  expect(screen.getByLabelText("迁移到哪个 provider")).toHaveValue("yihubangg");
  expect(screen.getByText("选择要迁移的会话")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /预览迁移/ })).toBeInTheDocument();
  expect(screen.getByText("默认只预览。确认迁移前会自动创建备份，所有数据都留在本机。")).toBeInTheDocument();
});

test("filters sessions by source provider", async () => {
  const user = userEvent.setup();
  render(<App />);

  await user.type(screen.getByLabelText("从哪个 provider 迁出"), "gmn");

  expect(screen.getByText("桌面版迁移助手产品设计")).toBeInTheDocument();
  expect(screen.queryByText("恢复 provider 切换后的会话")).not.toBeInTheDocument();
});

test("updates selected count when a session is unchecked", async () => {
  const user = userEvent.setup();
  render(<App />);

  expect(screen.getByRole("heading", { name: "准备迁移 3 个会话" })).toBeInTheDocument();

  const firstSession = screen.getByLabelText("选择会话：恢复 provider 切换后的会话");
  await user.click(firstSession);

  expect(screen.getByRole("heading", { name: "准备迁移 2 个会话" })).toBeInTheDocument();
});

test("keeps technical information behind advanced details", async () => {
  const user = userEvent.setup();
  render(<App />);

  expect(screen.queryByText(/019eca3b/)).not.toBeInTheDocument();

  const row = screen.getByRole("article", { name: "讨论会话迁移工具开源方向" });
  await user.click(within(row).getByRole("button", { name: "查看高级信息" }));

  expect(screen.getAllByText(/019ec94d/).length).toBeGreaterThan(0);
  expect(screen.getByText(/funai -> yihubangg/)).toBeInTheDocument();
});
