import {
  defineConfig,
  tableRequirements,
  singleIdVerification,
} from "@lapismd/spec-validator";
import { automationExempt } from "./scripts/ci-automation.mjs";
const config = defineConfig(tableRequirements(), {
  name: "lapis-ai",
  idPattern: /^LAI-[A-Z]+-\d{3}$/,
  tableSection: "Requirements",
  ruleIds: {
    fileLimits: "LAI-GOV-101",
    summary: "LAI-GOV-101",
    governance: "LAI-GOV-100",
    verification: "LAI-GOV-101",
    book: "LAI-GOV-101",
    bookIgnore: "LAI-GOV-101",
    specFirst: "LAI-GOV-100",
    internal: "LAI-GOV-100",
  },
  validators: {
    fileLimits: { maxLines: 400, maxBytes: 32768 },
    summary: true,
    governance: {
      extras: ["AGENTS.md", "README.md"],
      normative: true,
      proseLimits: false,
      acceptance: false,
      references: true,
      changeMap: false,
    },
    verification: {
      ...singleIdVerification({
        headers: {
          ids: ["ID"],
          status: ["Status"],
          evidence: ["Evidence"],
          required: [],
        },
        statuses: ["Implemented", "Planned"],
      }),
      files: ["^current/verification\\.md$"],
    },
    book: true,
    specFirst: {
      mode: "mapped",
      canonicalPattern: "^spec/src/current/(?:contracts/.+|governance)\\.md$",
      ignore: [
        "^spec/src/history/",
        "^spec/src/current/(?:verification|progress)/",
      ],
      protected: [
        "^(?:\\.github/|crates/|packages/|scripts/|examples/|Cargo|rust|package|pnpm|AGENTS|README|docs/|spec-validator|release\\.toml|CHANGELOG|\\.changeset/)",
      ],
      rules: [
        {
          pattern: "^packages/ai/",
          chapters: ["spec/src/current/contracts/tauri.md"],
        },
        {
          pattern: "^crates/tauri-plugin-lapis-ai/",
          chapters: ["spec/src/current/contracts/tauri.md"],
        },
        {
          pattern: "^crates/lapis-acp-protocol/",
          chapters: ["spec/src/current/contracts/acp/protocol.md"],
        },
        {
          pattern: "^crates/lapis-acp-agent/",
          chapters: ["spec/src/current/contracts/acp/agents.md"],
        },
        {
          pattern: "^crates/lapis-acp-runner/",
          chapters: ["spec/src/current/contracts/acp/runner.md"],
        },
        {
          pattern: "^crates/lapis-acp-control/",
          chapters: ["spec/src/current/contracts/acp/control.md"],
        },
        {
          pattern: "^crates/lapis-acp-cli/",
          chapters: ["spec/src/current/contracts/acp/cli.md"],
        },
        {
          pattern: "^crates/lapis-acp-testkit/",
          chapters: ["spec/src/current/contracts/acp/compatibility.md"],
        },
        {
          pattern: "^crates/lapis-(?:agent|agent-tools|compute)/",
          chapters: ["spec/src/current/contracts/services.md"],
        },
        {
          pattern: "^(?:crates/lapis-ai-|examples/)",
          chapters: ["spec/src/current/contracts/runtime.md"],
        },
        {
          pattern:
            "^(?:\\.github/|scripts/|Cargo|rust|package|pnpm|AGENTS|README|docs/|spec-validator|release\\.toml|CHANGELOG|\\.changeset/)",
          chapters: ["spec/src/current/governance.md"],
        },
      ],
    },
  },
  check: {
    first: true,
    build: true,
    lanes: [
      { name: "release", command: "pnpm", args: ["release:check"] },
      { name: "format", command: "pnpm", args: ["format:check"] },
      { name: "architecture", command: "pnpm", args: ["check:architecture"] },
      { name: "boundaries", command: "pnpm", args: ["check:boundaries"] },
      { name: "progress", command: "pnpm", args: ["check:progress"] },
      { name: "upstream", command: "pnpm", args: ["check:upstream"] },
      { name: "governance", command: "pnpm", args: ["test:governance"] },
    ],
  },
});

if (automationExempt()) config.validators.specFirst = false;
export default config;
