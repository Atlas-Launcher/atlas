import { spawnSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

interface RunOptions {
  cwd?: string;
  allowFailure?: boolean;
  capture?: boolean;
}

function run(command: string, args: string[], options: RunOptions = {}) {
  const result = spawnSync(command, args, {
    cwd: options.cwd,
    stdio: options.capture ? "pipe" : "inherit",
    encoding: "utf8"
  });
  if (result.error) {
    throw result.error;
  }
  if (!options.allowFailure && result.status !== 0) {
    throw new Error(
      `Command failed (${result.status ?? "unknown"}): ${command} ${args.join(" ")}`
    );
  }
  return result;
}

function usage() {
  console.log(`Usage:
  pnpm release:launcher -- <version>

Examples:
  pnpm release:launcher -- 0.2.0
  pnpm release:launcher -- launcher-v0.2.0

Requirements:
  - Run from a clean git working tree
  - gh CLI authenticated (gh auth login)
  - TAURI_SIGNING_PRIVATE_KEY set for the tauri build step
`);
}

function gatherArtifacts(bundleDir: string) {
  const artifacts: string[] = [];
  const stack = [bundleDir];

  while (stack.length > 0) {
    const current = stack.pop();
    if (!current) {
      continue;
    }
    const entries = readdirSync(current, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) {
        if (entry.name.endsWith(".dSYM")) {
          continue;
        }
        stack.push(fullPath);
        continue;
      }
      artifacts.push(fullPath);
    }
  }

  return artifacts.sort();
}

function main() {
  const rawArgs = process.argv.slice(2).filter((arg) => arg !== "--");
  const firstArg = rawArgs[0];
  if (!firstArg || firstArg === "--help" || firstArg === "-h") {
    usage();
    process.exit(firstArg ? 0 : 1);
  }

  const version = firstArg.replace(/^launcher-v/, "");
  if (!version) {
    throw new Error("Version is empty.");
  }
  const tag = `launcher-v${version}`;

  const scriptDir = path.dirname(fileURLToPath(import.meta.url));
  const launcherRoot = path.resolve(scriptDir, "..");
  const repoRoot = path.resolve(launcherRoot, "../..");
  const bundleDir = path.join(
    launcherRoot,
    "src-tauri",
    "target",
    "release",
    "bundle"
  );

  if (!process.env.TAURI_SIGNING_PRIVATE_KEY) {
    throw new Error(
      "TAURI_SIGNING_PRIVATE_KEY is not set. Export it before running this command."
    );
  }

  run("gh", ["auth", "status", "-h", "github.com"], { cwd: repoRoot });

  const workingTreeDirty =
    run("git", ["status", "--porcelain"], {
      cwd: repoRoot,
      allowFailure: true,
      capture: true
    }).stdout.trim().length > 0;
  if (workingTreeDirty) {
    throw new Error("Git working tree is not clean. Commit or stash changes first.");
  }

  const hasLocalTag = run("git", ["rev-parse", tag], {
    cwd: repoRoot,
    allowFailure: true,
    capture: true
  }).status === 0;
  if (hasLocalTag) {
    throw new Error(`Local tag '${tag}' already exists.`);
  }

  const hasRemoteTag =
    run("git", ["ls-remote", "--exit-code", "--tags", "origin", `refs/tags/${tag}`], {
      cwd: repoRoot,
      allowFailure: true,
      capture: true
    }).status === 0;
  if (hasRemoteTag) {
    throw new Error(`Remote tag '${tag}' already exists.`);
  }

  if (!existsSync(bundleDir)) {
    throw new Error(
      `Bundle directory not found: ${bundleDir}\nRun 'pnpm turbo run tauri:build --filter=atlas' first or use the root release command.`
    );
  }

  const artifacts = gatherArtifacts(bundleDir);
  if (artifacts.length === 0) {
    throw new Error(`No release artifacts found in ${bundleDir}`);
  }

  run("git", ["tag", tag], { cwd: repoRoot });
  run("git", ["push", "origin", tag], { cwd: repoRoot });

  run(
    "gh",
    [
      "release",
      "create",
      tag,
      "--title",
      `Atlas Launcher v${version}`,
      "--generate-notes",
      ...artifacts
    ],
    { cwd: repoRoot }
  );

  console.log(`Launcher release complete: ${tag}`);
}

try {
  main();
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`release:launcher failed: ${message}`);
  process.exit(1);
}
