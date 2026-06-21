import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, parse, resolve } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const tauriArgs = process.argv.slice(2);
const scriptDirectory = dirname(fileURLToPath(import.meta.url));

if (tauriArgs.length === 0) {
  console.error("Usage: node scripts/tauri-msvc-env.mjs <tauri-command> [...args]");
  process.exit(1);
}

if (process.platform !== "win32") {
  run(tauriCommand(), tauriArgs, { shell: process.platform === "win32" });
}

if (hasMsvcEnvironment()) {
  if (requiresWindowsSdk() && !hasWindowsSdkEnvironment()) {
    printWindowsSdkMissing();
    process.exit(1);
  }
  run(tauriCommand(), tauriArgs, { shell: true });
}

const vcvars = findVcvars64();
if (!vcvars) {
  console.error("Visual Studio C++ build environment was not found.");
  console.error("Install Visual Studio Build Tools with the C++ desktop workload and a Windows SDK, then rerun this command.");
  process.exit(1);
}

const scriptDir = join(tmpdir(), "ai-session-migrator");
mkdirSync(scriptDir, { recursive: true });
const scriptPath = join(scriptDir, `tauri-msvc-${process.pid}.cmd`);
const command = `@echo off\r\nsetlocal EnableExtensions EnableDelayedExpansion\r\ncall "${vcvars}" amd64 >nul\r\nif errorlevel 1 exit /b %errorlevel%\r\n${windowsSdkCheck()}\r\n${quoteCmd(tauriCommand())} ${tauriArgs.map(quoteCmdArg).join(" ")}\r\n`;
writeFileSync(scriptPath, command, "utf8");

try {
  run("cmd.exe", ["/d", "/c", scriptPath], { shell: false });
} finally {
  rmSync(scriptPath, { force: true });
}

function hasMsvcEnvironment() {
  return Boolean(process.env.VCToolsInstallDir && process.env.INCLUDE && process.env.LIB);
}

function hasWindowsSdkEnvironment() {
  const windowsSdkDir = process.env.WindowsSdkDir;
  if (!windowsSdkDir || !existsSync(join(windowsSdkDir, "Lib"))) {
    return false;
  }

  const libPaths = process.env.LIB?.split(";") ?? [];
  return libPaths.some((libPath) => existsSync(join(libPath, "kernel32.lib")));
}

function findVcvars64() {
  const installPath = vswhereInstallPath();
  const candidates = [
    process.env.VSINSTALLDIR && join(process.env.VSINSTALLDIR, "VC", "Auxiliary", "Build", "vcvars64.bat"),
    installPath && join(installPath, "VC", "Auxiliary", "Build", "vcvars64.bat"),
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Professional\\VC\\Auxiliary\\Build\\vcvars64.bat",
    "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\Build\\vcvars64.bat"
  ].filter(Boolean);

  return candidates.find((candidate) => existsSync(candidate)) ?? null;
}

function vswhereInstallPath() {
  const programFilesX86 = process.env["ProgramFiles(x86)"];
  if (!programFilesX86) {
    return null;
  }

  const vswhere = join(programFilesX86, "Microsoft Visual Studio", "Installer", "vswhere.exe");
  if (!existsSync(vswhere)) {
    return null;
  }

  const result = spawnSync(
    vswhere,
    [
      "-latest",
      "-products",
      "*",
      "-requires",
      "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
      "-property",
      "installationPath"
    ],
    { encoding: "utf8" }
  );

  if (result.status !== 0) {
    return null;
  }

  return result.stdout.trim() || null;
}

function tauriCommand() {
  if (process.platform !== "win32") {
    return "tauri";
  }

  const candidates = tauriCommandCandidates();

  return candidates.find((candidate) => existsSync(candidate)) ?? "tauri";
}

function tauriCommandCandidates() {
  const startDirs = [
    process.cwd(),
    process.env.INIT_CWD,
    process.env.npm_config_local_prefix,
    join(scriptDirectory, ".."),
    join(scriptDirectory, "..", "..", "..")
  ]
    .filter(Boolean)
    .map((value) => resolve(value));

  const candidates = [];
  const seenStarts = new Set();
  for (const startDir of startDirs) {
    if (seenStarts.has(startDir)) {
      continue;
    }
    seenStarts.add(startDir);
    candidates.push(...tauriCommandCandidatesFrom(startDir));
  }
  return [...new Set(candidates)];
}

function tauriCommandCandidatesFrom(startDir) {
  const candidates = [];
  let current = startDir;
  const root = parse(current).root;
  while (true) {
    candidates.push(join(current, "node_modules", ".bin", "tauri.cmd"));
    if (current === root) {
      return candidates;
    }
    current = dirname(current);
  }
}

function windowsSdkCheck() {
  if (!requiresWindowsSdk()) {
    return "";
  }

  return [
    "if not defined WindowsSdkDir (",
    `  echo ${windowsSdkMissingSummary()} 1>&2`,
    `  echo ${windowsSdkMissingAction()} 1>&2`,
    "  exit /b 1",
    ")",
    "if not exist \"!WindowsSdkDir!Lib\" (",
    "  echo Windows SDK libraries were not found at !WindowsSdkDir!Lib. 1>&2",
    `  echo ${windowsSdkMissingAction()} 1>&2`,
    "  exit /b 1",
    ")",
    "set \"_AI_SESSION_MIGRATOR_KERNEL32=\"",
    "for /f \"delims=\" %%K in ('dir /b /s \"!WindowsSdkDir!Lib\\kernel32.lib\" 2^>nul') do if not defined _AI_SESSION_MIGRATOR_KERNEL32 set \"_AI_SESSION_MIGRATOR_KERNEL32=%%K\"",
    "if not defined _AI_SESSION_MIGRATOR_KERNEL32 (",
    "  echo Windows SDK kernel32.lib was not found under !WindowsSdkDir!Lib. 1>&2",
    `  echo ${windowsSdkMissingAction()} 1>&2`,
    "  exit /b 1",
    ")"
  ].join("\r\n");
}

function requiresWindowsSdk() {
  return ["dev", "build"].includes(tauriArgs[0]);
}

function printWindowsSdkMissing() {
  console.error(windowsSdkMissingSummary());
  console.error(windowsSdkMissingAction());
}

function windowsSdkMissingSummary() {
  return "Windows SDK was not found.";
}

function windowsSdkMissingAction() {
  return "Install the Windows SDK component in Visual Studio Installer, then rerun this command.";
}

function run(command, args, options) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    env: process.env,
    ...options
  });

  if (result.error) {
    console.error(result.error.message);
    process.exit(1);
  }

  if (result.signal) {
    process.kill(process.pid, result.signal);
  }

  process.exit(result.status ?? 1);
}

function quoteCmd(value) {
  return `"${String(value).replace(/"/g, '""')}"`;
}

function quoteCmdArg(value) {
  const text = String(value);
  if (/^[a-zA-Z0-9_./:-]+$/.test(text)) {
    return text;
  }
  return quoteCmd(text);
}
