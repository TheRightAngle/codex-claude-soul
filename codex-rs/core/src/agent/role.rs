//! Applies agent-role configuration layers on top of an existing session config.
//!
//! Roles are selected at spawn time and are loaded with the same config machinery as
//! `config.toml`. This module resolves built-in and user-defined role files, inserts the role as a
//! high-precedence layer, and preserves the caller's current profile/provider unless the role
//! explicitly takes ownership of model selection. It does not decide when to spawn a sub-agent or
//! which role to use; the multi-agent tool handler owns that orchestration.

use crate::config::AgentRoleConfig;
use crate::config::Config;
use crate::config::ConfigOverrides;
use crate::config::agent_roles::parse_agent_role_file_contents;
use crate::config::deserialize_config_toml_with_base;
use crate::config_loader::ConfigLayerEntry;
use crate::config_loader::ConfigLayerStack;
use crate::config_loader::ConfigLayerStackOrdering;
use crate::config_loader::resolve_relative_paths_in_config_toml;
use anyhow::anyhow;
use codex_app_server_protocol::ConfigLayerSource;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::LazyLock;
use toml::Value as TomlValue;

/// The role name used when a caller omits `agent_type`.
pub const DEFAULT_ROLE_NAME: &str = "default";
const AGENT_TYPE_UNAVAILABLE_ERROR: &str = "agent type is currently not available";

/// Applies a named role layer to `config` while preserving caller-owned model selection.
///
/// The role layer is inserted at session-flag precedence so it can override persisted config, but
/// the caller's current `profile` and `model_provider` remain sticky runtime choices unless the
/// role explicitly sets `profile`, explicitly sets `model_provider`, or rewrites the active
/// profile's `model_provider` in place. Rebuilding the config without those overrides would make a
/// spawned agent silently fall back to the default provider, which is the bug this preservation
/// logic avoids.
pub(crate) async fn apply_role_to_config(
    config: &mut Config,
    role_name: Option<&str>,
) -> Result<(), String> {
    let role_name = role_name.unwrap_or(DEFAULT_ROLE_NAME);

    let role = resolve_role_config(config, role_name)
        .cloned()
        .ok_or_else(|| format!("unknown agent_type '{role_name}'"))?;

    apply_role_to_config_inner(config, role_name, &role)
        .await
        .map_err(|err| {
            tracing::warn!("failed to apply role to config: {err}");
            AGENT_TYPE_UNAVAILABLE_ERROR.to_string()
        })
}

async fn apply_role_to_config_inner(
    config: &mut Config,
    role_name: &str,
    role: &AgentRoleConfig,
) -> anyhow::Result<()> {
    let is_built_in = !config.agent_roles.contains_key(role_name);
    let Some(config_file) = role.config_file.as_ref() else {
        return Ok(());
    };
    let role_layer_toml = load_role_layer_toml(config, config_file, is_built_in, role_name).await?;
    let (preserve_current_profile, preserve_current_provider) =
        preservation_policy(config, &role_layer_toml);

    *config = reload::build_next_config(
        config,
        role_layer_toml,
        preserve_current_profile,
        preserve_current_provider,
    )?;
    Ok(())
}

async fn load_role_layer_toml(
    config: &Config,
    config_file: &Path,
    is_built_in: bool,
    role_name: &str,
) -> anyhow::Result<TomlValue> {
    let (role_config_toml, role_config_base) = if is_built_in {
        let role_config_contents = built_in::config_file_contents(config_file)
            .map(str::to_owned)
            .ok_or(anyhow!("No corresponding config content"))?;
        let role_config_toml: TomlValue = toml::from_str(&role_config_contents)?;
        (role_config_toml, config.codex_home.as_path())
    } else {
        let role_config_contents = tokio::fs::read_to_string(config_file).await?;
        let role_config_base = config_file
            .parent()
            .ok_or(anyhow!("No corresponding config content"))?;
        let role_config_toml = parse_agent_role_file_contents(
            &role_config_contents,
            config_file,
            role_config_base,
            Some(role_name),
        )?
        .config;
        (role_config_toml, role_config_base)
    };

    deserialize_config_toml_with_base(role_config_toml.clone(), role_config_base)?;
    Ok(resolve_relative_paths_in_config_toml(
        role_config_toml,
        role_config_base,
    )?)
}

pub(crate) fn resolve_role_config<'a>(
    config: &'a Config,
    role_name: &str,
) -> Option<&'a AgentRoleConfig> {
    config
        .agent_roles
        .get(role_name)
        .or_else(|| built_in::configs().get(role_name))
}

fn preservation_policy(config: &Config, role_layer_toml: &TomlValue) -> (bool, bool) {
    let role_selects_provider = role_layer_toml.get("model_provider").is_some();
    let role_selects_profile = role_layer_toml.get("profile").is_some();
    let role_updates_active_profile_provider = config
        .active_profile
        .as_ref()
        .and_then(|active_profile| {
            role_layer_toml
                .get("profiles")
                .and_then(TomlValue::as_table)
                .and_then(|profiles| profiles.get(active_profile))
                .and_then(TomlValue::as_table)
                .map(|profile| profile.contains_key("model_provider"))
        })
        .unwrap_or(false);
    let preserve_current_profile = !role_selects_provider && !role_selects_profile;
    let preserve_current_provider =
        preserve_current_profile && !role_updates_active_profile_provider;
    (preserve_current_profile, preserve_current_provider)
}

mod reload {
    use super::*;

    pub(super) fn build_next_config(
        config: &Config,
        role_layer_toml: TomlValue,
        preserve_current_profile: bool,
        preserve_current_provider: bool,
    ) -> anyhow::Result<Config> {
        let active_profile_name = preserve_current_profile
            .then_some(config.active_profile.as_deref())
            .flatten();
        let config_layer_stack =
            build_config_layer_stack(config, &role_layer_toml, active_profile_name)?;
        let mut merged_config = deserialize_effective_config(config, &config_layer_stack)?;
        if preserve_current_profile {
            merged_config.profile = None;
        }

        let mut next_config = Config::load_config_with_layer_stack(
            merged_config,
            reload_overrides(config, preserve_current_provider),
            config.codex_home.clone(),
            config_layer_stack,
        )?;
        if preserve_current_profile {
            next_config.active_profile = config.active_profile.clone();
        }
        Ok(next_config)
    }

    fn build_config_layer_stack(
        config: &Config,
        role_layer_toml: &TomlValue,
        active_profile_name: Option<&str>,
    ) -> anyhow::Result<ConfigLayerStack> {
        let mut layers = existing_layers(config);
        if let Some(resolved_profile_layer) =
            resolved_profile_layer(config, &layers, role_layer_toml, active_profile_name)?
        {
            insert_layer(&mut layers, resolved_profile_layer);
        }
        insert_layer(&mut layers, role_layer(role_layer_toml.clone()));
        Ok(ConfigLayerStack::new(
            layers,
            config.config_layer_stack.requirements().clone(),
            config.config_layer_stack.requirements_toml().clone(),
        )?)
    }

    fn resolved_profile_layer(
        config: &Config,
        existing_layers: &[ConfigLayerEntry],
        role_layer_toml: &TomlValue,
        active_profile_name: Option<&str>,
    ) -> anyhow::Result<Option<ConfigLayerEntry>> {
        let Some(active_profile_name) = active_profile_name else {
            return Ok(None);
        };

        let mut layers = existing_layers.to_vec();
        insert_layer(&mut layers, role_layer(role_layer_toml.clone()));
        let merged_config = deserialize_effective_config(
            config,
            &ConfigLayerStack::new(
                layers,
                config.config_layer_stack.requirements().clone(),
                config.config_layer_stack.requirements_toml().clone(),
            )?,
        )?;
        let resolved_profile =
            merged_config.get_config_profile(Some(active_profile_name.to_string()))?;
        Ok(Some(ConfigLayerEntry::new(
            ConfigLayerSource::SessionFlags,
            TomlValue::try_from(resolved_profile)?,
        )))
    }

    fn deserialize_effective_config(
        config: &Config,
        config_layer_stack: &ConfigLayerStack,
    ) -> anyhow::Result<crate::config::ConfigToml> {
        Ok(deserialize_config_toml_with_base(
            config_layer_stack.effective_config(),
            &config.codex_home,
        )?)
    }

    fn existing_layers(config: &Config) -> Vec<ConfigLayerEntry> {
        config
            .config_layer_stack
            .get_layers(
                ConfigLayerStackOrdering::LowestPrecedenceFirst,
                /*include_disabled*/ true,
            )
            .into_iter()
            .cloned()
            .collect()
    }

    fn insert_layer(layers: &mut Vec<ConfigLayerEntry>, layer: ConfigLayerEntry) {
        let insertion_index =
            layers.partition_point(|existing_layer| existing_layer.name <= layer.name);
        layers.insert(insertion_index, layer);
    }

    fn role_layer(role_layer_toml: TomlValue) -> ConfigLayerEntry {
        ConfigLayerEntry::new(ConfigLayerSource::SessionFlags, role_layer_toml)
    }

    fn reload_overrides(config: &Config, preserve_current_provider: bool) -> ConfigOverrides {
        ConfigOverrides {
            cwd: Some(config.cwd.to_path_buf()),
            model_provider: preserve_current_provider.then(|| config.model_provider_id.clone()),
            codex_linux_sandbox_exe: config.codex_linux_sandbox_exe.clone(),
            main_execve_wrapper_exe: config.main_execve_wrapper_exe.clone(),
            js_repl_node_path: config.js_repl_node_path.clone(),
            ..Default::default()
        }
    }
}

pub(crate) mod spawn_tool_spec {
    use super::*;

    /// Builds the spawn-agent tool description text from built-in and configured roles.
    pub(crate) fn build(user_defined_agent_roles: &BTreeMap<String, AgentRoleConfig>) -> String {
        let built_in_roles = built_in::configs();
        build_from_configs(built_in_roles, user_defined_agent_roles)
    }

    // This function is not inlined for testing purpose.
    fn build_from_configs(
        built_in_roles: &BTreeMap<String, AgentRoleConfig>,
        user_defined_roles: &BTreeMap<String, AgentRoleConfig>,
    ) -> String {
        let mut seen = BTreeSet::new();
        let mut formatted_roles = Vec::new();
        for (name, declaration) in user_defined_roles {
            if seen.insert(name.as_str()) {
                formatted_roles.push(format_role(name, declaration));
            }
        }
        for (name, declaration) in built_in_roles {
            if seen.insert(name.as_str()) {
                formatted_roles.push(format_role(name, declaration));
            }
        }

        format!(
            "Optional type name for the new agent. If omitted, `{DEFAULT_ROLE_NAME}` is used.\nAvailable roles:\n{}",
            formatted_roles.join("\n"),
        )
    }

    fn format_role(name: &str, declaration: &AgentRoleConfig) -> String {
        if let Some(description) = &declaration.description {
            let locked_settings_note = declaration
                .config_file
                .as_ref()
                .and_then(|config_file| {
                    built_in::config_file_contents(config_file)
                        .map(str::to_owned)
                        .or_else(|| std::fs::read_to_string(config_file).ok())
                })
                .and_then(|contents| toml::from_str::<TomlValue>(&contents).ok())
                .map(|role_toml| {
                    let model = role_toml
                        .get("model")
                        .and_then(TomlValue::as_str);
                    let reasoning_effort = role_toml
                        .get("model_reasoning_effort")
                        .and_then(TomlValue::as_str);

                    match (model, reasoning_effort) {
                        (Some(model), Some(reasoning_effort)) => format!(
                            "\n- This role's model is set to `{model}` and its reasoning effort is set to `{reasoning_effort}`. These settings cannot be changed."
                        ),
                        (Some(model), None) => {
                            format!(
                                "\n- This role's model is set to `{model}` and cannot be changed."
                            )
                        }
                        (None, Some(reasoning_effort)) => {
                            format!(
                                "\n- This role's reasoning effort is set to `{reasoning_effort}` and cannot be changed."
                            )
                        }
                        (None, None) => String::new(),
                    }
                })
                .unwrap_or_default();
            format!("{name}: {{\n{description}{locked_settings_note}\n}}")
        } else {
            format!("{name}: no description")
        }
    }
}

mod built_in {
    use super::*;

    /// Returns the cached built-in role declarations defined in this module.
    pub(super) fn configs() -> &'static BTreeMap<String, AgentRoleConfig> {
        static CONFIG: LazyLock<BTreeMap<String, AgentRoleConfig>> = LazyLock::new(|| {
            /// Helper to build a config_file PathBuf from a TOML filename.
            fn toml_path(name: &str) -> Option<std::path::PathBuf> {
                Some(name.to_string().parse().unwrap_or_default())
            }

            BTreeMap::from([
                // ── Core agents ──────────────────────────────────────────
                (
                    DEFAULT_ROLE_NAME.to_string(),
                    AgentRoleConfig {
                        description: Some("Default agent.".to_string()),
                        config_file: None,
                        nickname_candidates: None,
                    },
                ),
                (
                    "explorer".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `explorer` for specific codebase questions.
Explorers are fast and authoritative.
They must be used to ask specific, well-scoped questions on the codebase.
Rules:
- In order to avoid redundant work, you should avoid exploring the same problem that explorers have already covered. Typically, you should trust the explorer results without additional verification. You are still allowed to inspect the code yourself to gain the needed context!
- You are encouraged to spawn up multiple explorers in parallel when you have multiple distinct questions to ask about the codebase that can be answered independently. This allows you to get more information faster without waiting for one question to finish before asking the next. While waiting for the explorer results, you can continue working on other local tasks that do not depend on those results. This parallelism is a key advantage of delegation, so use it whenever you have multiple questions to ask.
- Reuse existing explorers for related questions."#
                                .to_string(),
                        ),
                        config_file: toml_path("explorer.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "planner".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `planner` for designing implementation approaches.
Planners are read-only and produce actionable plans with file paths, specific changes, and verification steps.
Rules:
- Plans should be concise — hard limit of 40 lines.
- Reference specific files and line numbers.
- List reusable existing functions and patterns.
- End with a verification strategy."#
                                .to_string(),
                        ),
                        config_file: toml_path("planner.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "general_purpose".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `general_purpose` for complex, multi-step tasks.
General-purpose agents are autonomous and can search, read, write, and execute.
Rules:
- Keeps going until the task is fully resolved.
- Prefers editing existing files over creating new ones.
- Uses absolute file paths."#
                                .to_string(),
                        ),
                        config_file: toml_path("general_purpose.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "researcher".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `researcher` for targeted research questions.
Researchers are efficient and evidence-based.
Rules:
- Focused on specific questions — no tangential exploration.
- Reports what was found AND what was not found.
- Cross-references multiple sources."#
                                .to_string(),
                        ),
                        config_file: toml_path("researcher.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "worker".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use for execution and production work.
Typical tasks:
- Implement part of a feature
- Fix tests or bugs
- Split large refactors into independent chunks
Rules:
- Explicitly assign **ownership** of the task (files / responsibility). When the subtask involves code changes, you should clearly specify which files or modules the worker is responsible for. This helps avoid merge conflicts and ensures accountability. For example, you can say "Worker 1 is responsible for updating the authentication module, while Worker 2 will handle the database layer." By defining clear ownership, you can delegate more effectively and reduce coordination overhead.
- Always tell workers they are **not alone in the codebase**, and they should not revert the edits made by others, and they should adjust their implementation to accommodate the changes made by others. This is important because there may be multiple workers making changes in parallel, and they need to be aware of each other's work to avoid conflicts and ensure a cohesive final product."#
                                .to_string(),
                        ),
                        config_file: None,
                        nickname_candidates: None,
                    },
                ),
                (
                    "worker_fork".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `worker_fork` for isolated execution of specific directives.
Worker forks execute silently and commit before reporting.
Rules:
- Does NOT spawn sub-agents — works directly with tools.
- Stays within assigned scope.
- Reports results in under 500 words."#
                                .to_string(),
                        ),
                        config_file: toml_path("worker_fork.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "awaiter".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `awaiter` EVERY TIME you must run a command that will take a very long time.
This includes testing, monitoring of a long running process, or an explicit ask to wait.
Rules:
- When an awaiter is running, you can work on something else. If you need to wait for its completion, use the largest possible timeout.
- Be patient with the `awaiter`.
- Do not use an awaiter for every compilation/test if it won't take time. Only use for long running commands.
- Close the awaiter when you're done with it."#
                                .to_string(),
                        ),
                        config_file: toml_path("awaiter.toml"),
                        nickname_candidates: None,
                    },
                ),
                // ── Verification & security ──────────────────────────────
                (
                    "verifier".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `verifier` for independent verification that work is correct and complete.
Verifiers are adversarial — they actively try to find failures.
Rules:
- Must run actual commands, not assume correctness.
- Uses type-specific verification strategies (frontend, backend, CLI, etc.).
- Reports PASS / FAIL / PARTIAL with evidence."#
                                .to_string(),
                        ),
                        config_file: toml_path("verifier.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "security_monitor".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: reviews autonomous actions for security risks. \
                             Classifies actions as ALLOWED / BLOCKED / NEEDS_REVIEW \
                             against 24 block rules and 6 allow exceptions."
                                .to_string(),
                        ),
                        config_file: toml_path("security_monitor.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "security_review".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `security_review` for code security audits.
Analyzes changes for exploitable vulnerabilities across OWASP categories.
Reports severity, confidence, exploit scenario, and fix for each finding."#
                                .to_string(),
                        ),
                        config_file: toml_path("security_review.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "auto_mode_reviewer".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: reviews auto mode classifier rules for clarity, \
                             completeness, conflicts, and actionability."
                                .to_string(),
                        ),
                        config_file: toml_path("auto_mode_reviewer.toml"),
                        nickname_candidates: None,
                    },
                ),
                // ── Git & PR ─────────────────────────────────────────────
                (
                    "quick_commit".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use `quick_commit` for single commit creation. \
                             Analyzes changes, drafts commit message, stages files by name, \
                             and creates the commit following git safety protocol."
                                .to_string(),
                        ),
                        config_file: toml_path("quick_commit.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "quick_pr".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use `quick_pr` for creating commits and pull requests. \
                             Commits changes, pushes, and creates a PR with summary and test plan."
                                .to_string(),
                        ),
                        config_file: toml_path("quick_pr.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "pr_comments".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use `pr_comments` to fetch and display GitHub PR comments. \
                             Shows inline and PR-level comments grouped by thread."
                                .to_string(),
                        ),
                        config_file: toml_path("pr_comments.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "review_pr".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `review_pr` for code review of GitHub pull requests.
Reviews correctness, conventions, performance, test coverage, and security.
Reports issues with exact file/line, severity, and suggested fix."#
                                .to_string(),
                        ),
                        config_file: toml_path("review_pr.toml"),
                        nickname_candidates: None,
                    },
                ),
                // ── Session management ───────────────────────────────────
                (
                    "title_generator".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: generates concise session titles (3-7 words, JSON)."
                                .to_string(),
                        ),
                        config_file: toml_path("title_generator.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "title_branch_generator".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: generates session title and git branch name (JSON)."
                                .to_string(),
                        ),
                        config_file: toml_path("title_branch_generator.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "conversation_summarizer".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: creates detailed 9-section conversation summaries."
                                .to_string(),
                        ),
                        config_file: toml_path("conversation_summarizer.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "recent_summarizer".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: summarizes recent conversation (post-compaction)."
                                .to_string(),
                        ),
                        config_file: toml_path("recent_summarizer.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "session_search".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: finds relevant sessions matching a user query."
                                .to_string(),
                        ),
                        config_file: toml_path("session_search.toml"),
                        nickname_candidates: None,
                    },
                ),
                // ── Content processing ───────────────────────────────────
                (
                    "webfetch_summarizer".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: summarizes fetched web content with trust-level rules."
                                .to_string(),
                        ),
                        config_file: toml_path("webfetch_summarizer.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "bash_description".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: generates concise command descriptions in active voice."
                                .to_string(),
                        ),
                        config_file: toml_path("bash_description.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "bash_prefix_detection".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: extracts command prefix and detects injection."
                                .to_string(),
                        ),
                        config_file: toml_path("bash_prefix_detection.toml"),
                        nickname_candidates: None,
                    },
                ),
                // ── Infrastructure ───────────────────────────────────────
                (
                    "hook_evaluator".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: evaluates hook conditions (JSON ok/not-ok output)."
                                .to_string(),
                        ),
                        config_file: toml_path("hook_evaluator.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "agent_hook".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: verifies stop conditions against conversation transcript."
                                .to_string(),
                        ),
                        config_file: toml_path("agent_hook.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "batch_orchestrator".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `batch_orchestrator` for large, parallelizable changes.
Decomposes work into 5-30 independent units, spawns workers in parallel, tracks progress.
Rules:
- Each unit must be independently committable.
- Never spawn more than 30 workers.
- Shows final summary with pass/fail counts."#
                                .to_string(),
                        ),
                        config_file: toml_path("batch_orchestrator.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "codex_guide".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            r#"Use `codex_guide` for help with Codex CLI, Agent SDK, and API usage.
Fetches official documentation and provides actionable guidance.
Covers slash commands, settings, hooks, AGENTS.md conventions, SDK examples, and API usage."#
                                .to_string(),
                        ),
                        config_file: toml_path("codex_guide.toml"),
                        nickname_candidates: None,
                    },
                ),
                // ── Creation & setup ─────────────────────────────────────
                (
                    "agent_architect".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use `agent_architect` to design custom agent configurations. \
                             Translates requirements into JSON with identifier, whenToUse, \
                             and systemPrompt."
                                .to_string(),
                        ),
                        config_file: toml_path("agent_architect.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "agentsmd_creation".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "Use `agentsmd_creation` to analyze a codebase and create AGENTS.md. \
                             Discovers build/test/lint commands and documents architecture."
                                .to_string(),
                        ),
                        config_file: toml_path("agentsmd_creation.toml"),
                        nickname_candidates: None,
                    },
                ),
                (
                    "suggestion_generator".to_string(),
                    AgentRoleConfig {
                        description: Some(
                            "System agent: predicts user's natural next input (2-12 words)."
                                .to_string(),
                        ),
                        config_file: toml_path("suggestion_generator.toml"),
                        nickname_candidates: None,
                    },
                ),
            ])
        });
        &CONFIG
    }

    /// Resolves a built-in role `config_file` path to embedded content.
    pub(super) fn config_file_contents(path: &Path) -> Option<&'static str> {
        // Core agents
        const EXPLORER: &str = include_str!("builtins/explorer.toml");
        const PLANNER: &str = include_str!("builtins/planner.toml");
        const GENERAL_PURPOSE: &str = include_str!("builtins/general_purpose.toml");
        const RESEARCHER: &str = include_str!("builtins/researcher.toml");
        const WORKER_FORK: &str = include_str!("builtins/worker_fork.toml");
        const AWAITER: &str = include_str!("builtins/awaiter.toml");
        // Verification & security
        const VERIFIER: &str = include_str!("builtins/verifier.toml");
        const SECURITY_MONITOR: &str = include_str!("builtins/security_monitor.toml");
        const SECURITY_REVIEW: &str = include_str!("builtins/security_review.toml");
        const AUTO_MODE_REVIEWER: &str = include_str!("builtins/auto_mode_reviewer.toml");
        // Git & PR
        const QUICK_COMMIT: &str = include_str!("builtins/quick_commit.toml");
        const QUICK_PR: &str = include_str!("builtins/quick_pr.toml");
        const PR_COMMENTS: &str = include_str!("builtins/pr_comments.toml");
        const REVIEW_PR: &str = include_str!("builtins/review_pr.toml");
        // Session management
        const TITLE_GENERATOR: &str = include_str!("builtins/title_generator.toml");
        const TITLE_BRANCH_GENERATOR: &str = include_str!("builtins/title_branch_generator.toml");
        const CONVERSATION_SUMMARIZER: &str =
            include_str!("builtins/conversation_summarizer.toml");
        const RECENT_SUMMARIZER: &str = include_str!("builtins/recent_summarizer.toml");
        const SESSION_SEARCH: &str = include_str!("builtins/session_search.toml");
        // Content processing
        const WEBFETCH_SUMMARIZER: &str = include_str!("builtins/webfetch_summarizer.toml");
        const BASH_DESCRIPTION: &str = include_str!("builtins/bash_description.toml");
        const BASH_PREFIX_DETECTION: &str = include_str!("builtins/bash_prefix_detection.toml");
        // Infrastructure
        const HOOK_EVALUATOR: &str = include_str!("builtins/hook_evaluator.toml");
        const AGENT_HOOK: &str = include_str!("builtins/agent_hook.toml");
        const BATCH_ORCHESTRATOR: &str = include_str!("builtins/batch_orchestrator.toml");
        const CODEX_GUIDE: &str = include_str!("builtins/codex_guide.toml");
        // Creation & setup
        const AGENT_ARCHITECT: &str = include_str!("builtins/agent_architect.toml");
        const AGENTSMD_CREATION: &str = include_str!("builtins/agentsmd_creation.toml");
        const SUGGESTION_GENERATOR: &str = include_str!("builtins/suggestion_generator.toml");

        match path.to_str()? {
            // Core agents
            "explorer.toml" => Some(EXPLORER),
            "planner.toml" => Some(PLANNER),
            "general_purpose.toml" => Some(GENERAL_PURPOSE),
            "researcher.toml" => Some(RESEARCHER),
            "worker_fork.toml" => Some(WORKER_FORK),
            "awaiter.toml" => Some(AWAITER),
            // Verification & security
            "verifier.toml" => Some(VERIFIER),
            "security_monitor.toml" => Some(SECURITY_MONITOR),
            "security_review.toml" => Some(SECURITY_REVIEW),
            "auto_mode_reviewer.toml" => Some(AUTO_MODE_REVIEWER),
            // Git & PR
            "quick_commit.toml" => Some(QUICK_COMMIT),
            "quick_pr.toml" => Some(QUICK_PR),
            "pr_comments.toml" => Some(PR_COMMENTS),
            "review_pr.toml" => Some(REVIEW_PR),
            // Session management
            "title_generator.toml" => Some(TITLE_GENERATOR),
            "title_branch_generator.toml" => Some(TITLE_BRANCH_GENERATOR),
            "conversation_summarizer.toml" => Some(CONVERSATION_SUMMARIZER),
            "recent_summarizer.toml" => Some(RECENT_SUMMARIZER),
            "session_search.toml" => Some(SESSION_SEARCH),
            // Content processing
            "webfetch_summarizer.toml" => Some(WEBFETCH_SUMMARIZER),
            "bash_description.toml" => Some(BASH_DESCRIPTION),
            "bash_prefix_detection.toml" => Some(BASH_PREFIX_DETECTION),
            // Infrastructure
            "hook_evaluator.toml" => Some(HOOK_EVALUATOR),
            "agent_hook.toml" => Some(AGENT_HOOK),
            "batch_orchestrator.toml" => Some(BATCH_ORCHESTRATOR),
            "codex_guide.toml" => Some(CODEX_GUIDE),
            // Creation & setup
            "agent_architect.toml" => Some(AGENT_ARCHITECT),
            "agentsmd_creation.toml" => Some(AGENTSMD_CREATION),
            "suggestion_generator.toml" => Some(SUGGESTION_GENERATOR),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "role_tests.rs"]
mod tests;
