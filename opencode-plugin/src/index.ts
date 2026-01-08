/**
 * WM OpenCode Plugin
 *
 * Working memory for OpenCode. The LLM calls wm tools when it needs context.
 * Similar to superego's "pull mode" - model-driven rather than automatic.
 */

import type { Plugin } from "@opencode-ai/plugin";
import { tool } from "@opencode-ai/plugin";
import { existsSync, appendFileSync } from "fs";
import { join } from "path";
import { spawnSync } from "child_process";

// Log to file since OpenCode is a TUI
function log(wmDir: string, message: string): void {
  const timestamp = new Date().toISOString();
  const logFile = join(wmDir, "hook.log");
  try {
    appendFileSync(logFile, `${timestamp} ${message}\n`);
  } catch {
    // Ignore log failures
  }
}

const WM_DIR = ".wm";
const WM_CONTRACT = `WM ACTIVE: This project uses wm (working memory) to accumulate tacit knowledge across sessions. When you need relevant context for your work, use the wm tool to retrieve it. If you don't know what/why something works or need background, encourage the user to prep a dive pack via wm. Key commands: wm show state (view knowledge), wm compile (get relevant context for current task).`;

// Check if wm binary is available
function checkWmBinary(): { available: boolean; version?: string } {
  try {
    const result = spawnSync("wm", ["--version"], { encoding: "utf-8" });
    if (result.status === 0) {
      return { available: true, version: result.stdout.trim() };
    }
    return { available: false };
  } catch {
    return { available: false };
  }
}

// Execute wm command
function executeWmCommand(
  directory: string,
  args: string[]
): { success: boolean; output: string; error?: string } {
  try {
    const result = spawnSync("wm", args, {
      cwd: directory,
      encoding: "utf-8",
      timeout: 30000,
      maxBuffer: 10 * 1024 * 1024,
    });

    if (result.status === 0) {
      return { success: true, output: result.stdout || "Command completed successfully" };
    } else {
      return {
        success: false,
        output: result.stdout || "",
        error: result.stderr || `Command failed with exit code ${result.status}`,
      };
    }
  } catch (e) {
    return { success: false, output: "", error: `Failed to execute command: ${e}` };
  }
}

export const WM: Plugin = async ({ directory }) => {
  const wmDir = join(directory, WM_DIR);
  const initialized = existsSync(wmDir);
  const wmCheck = checkWmBinary();

  if (initialized && wmCheck.available) {
    log(wmDir, `Plugin loaded, binary: ${wmCheck.version}`);
  }

  return {
    // Inject contract into system prompt (soft hint for model to use tool)
    "experimental.chat.system.transform": async (_input, output) => {
      if (initialized && wmCheck.available) {
        output.system.push(WM_CONTRACT);
      }
    },

    tool: {
      wm: tool({
        description: "Manage working memory. Commands: init, status, show <state|working|sessions>, compile, distill, compress, pause, resume. Use 'compile' to get relevant context for current task.",
        args: {
          command: tool.schema
            .enum(["init", "status", "show", "compile", "distill", "compress", "pause", "resume"])
            .default("status"),
          subcommand: tool.schema.string().optional(),
          options: tool.schema.string().optional(),
        },
        async execute({ command, subcommand, options }) {
          // Check if wm binary is available
          if (!wmCheck.available) {
            return `wm binary not found. Install with:\n\n` +
                   `  brew tap cloud-atlas-ai/wm\n` +
                   `  brew install wm\n\n` +
                   `Or:\n\n` +
                   `  cargo install wm\n\n` +
                   `Then restart OpenCode.`;
          }

          // Build command arguments
          const args: string[] = [command];
          if (subcommand) args.push(subcommand);
          if (options) args.push(...options.split(/\s+/));

          // Special handling for init
          if (command === "init") {
            if (existsSync(wmDir)) {
              return "wm already initialized in this project.";
            }
            const result = executeWmCommand(directory, args);
            if (result.success) {
              return `wm initialized. Created ${WM_DIR}/ directory.\n\n` +
                     `Working memory will accumulate knowledge from your sessions.\n\n` +
                     `Key commands:\n` +
                     `  wm show state - View accumulated knowledge\n` +
                     `  wm compile - Get relevant context for current task\n` +
                     `  wm distill - Extract knowledge from recent work`;
            }
            return `Failed to initialize wm: ${result.error || result.output}`;
          }

          // Check if initialized for other commands
          if (!existsSync(wmDir)) {
            return "wm not initialized. Use 'wm init' first.";
          }

          // Execute command
          const result = executeWmCommand(directory, args);

          if (result.success) {
            // Special message for compile to explain output
            if (command === "compile") {
              return result.output + "\n\n(This is the relevant context from working memory for your current task)";
            }
            return result.output;
          } else {
            return `Error: ${result.error}\n\n${result.output}`;
          }
        },
      }),
    },
  };
};

export default WM;
