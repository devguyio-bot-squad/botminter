use std::collections::BTreeMap;

/// A skill available for loading on demand during a chat session.
#[derive(Debug, Clone)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub load_command: String,
}

/// Parameters for building a meta-prompt for `bm chat`.
pub struct MetaPromptParams<'a> {
    pub member_name: &'a str,
    pub role_name: &'a str,
    pub role_description: &'a str,
    pub team_name: &'a str,
    pub guardrails: &'a [String],
    pub hat_instructions: &'a BTreeMap<String, String>,
    pub prompt_md_content: &'a str,
    pub reference_dir: &'a str,
    pub hat: Option<&'a str>,
    pub skills: &'a [SkillInfo],
}

/// Builds a meta-prompt for an interactive `bm chat` session.
///
/// Assembles role identity, hat capabilities, guardrails, role context,
/// and reference paths into a single markdown document. Supports two modes:
/// - Hatless (hat=None): all hats' instructions included
/// - Hat-specific (hat=Some("executor")): only that hat's instructions
pub fn build_meta_prompt(params: &MetaPromptParams) -> String {
    let mut out = String::new();

    // Header: role identity
    out.push_str(&format!(
        "# Interactive Session — {}\n",
        params.member_name
    ));
    out.push('\n');
    out.push_str(&format!(
        "You are a member of the {} team.\n",
        params.team_name
    ));
    out.push_str(&format!("Your name is {}.\n", params.member_name));
    out.push_str(&format!("Your role is called {}.\n", params.role_name));
    if !params.role_description.is_empty() {
        out.push_str(&format!(
            "Your role description is: {}\n",
            params.role_description
        ));
    }
    out.push_str("You normally run autonomously inside Ralph Orchestrator.\n");
    out.push_str("Right now you are in an interactive session with the human (PO).\n");

    // Your Capabilities section
    out.push('\n');
    out.push_str("## Your Capabilities\n");
    out.push('\n');

    match params.hat {
        Some(hat_name) => {
            if let Some(instructions) = params.hat_instructions.get(hat_name) {
                out.push_str(instructions.trim_end());
                out.push('\n');
            }
        }
        None => {
            // Hatless mode: include all hats (BTreeMap gives sorted order)
            for (name, instructions) in params.hat_instructions {
                out.push_str(&format!("### {}\n", name));
                out.push('\n');
                out.push_str(instructions.trim_end());
                out.push_str("\n\n");
            }
        }
    }

    // Skills section (only if skills are available)
    if !params.skills.is_empty() {
        out.push_str("## Skills\n");
        out.push('\n');
        out.push_str("Available skills you can load on demand:\n");
        out.push('\n');
        out.push_str("| Skill | Description | Load Command |\n");
        out.push_str("|-------|-------------|---------------|\n");
        for skill in params.skills {
            out.push_str(&format!(
                "| {} | {} | Read `{}` |\n",
                skill.name, skill.description, skill.load_command
            ));
        }
        out.push('\n');
        out.push_str("To use a skill, read its SKILL.md file for full instructions.\n");
        out.push('\n');
    }

    // Guardrails section
    out.push_str("## Guardrails\n");
    out.push('\n');
    for (i, guardrail) in params.guardrails.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", 999 + i, guardrail));
    }

    // Role Context section
    out.push('\n');
    out.push_str("## Role Context\n");
    out.push('\n');
    out.push_str(params.prompt_md_content.trim_end());
    out.push('\n');

    // Reference section
    out.push('\n');
    out.push_str("## Reference: Operation Mode\n");
    out.push('\n');
    out.push_str(
        "When running autonomously inside Ralph Orchestrator, you follow the\n\
         operational workflows described in: ",
    );
    out.push_str(params.reference_dir);
    out.push('\n');
    out.push_str("These do not apply in interactive mode — the human drives the workflow.\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_params() -> (Vec<String>, BTreeMap<String, String>, String) {
        let guardrails = vec![
            "Always follow team invariants".to_string(),
            "Use gh CLI for all GitHub operations".to_string(),
        ];
        let mut hats = BTreeMap::new();
        hats.insert(
            "executor".to_string(),
            "You are the executor hat.\nPick up tasks and execute them.\n".to_string(),
        );
        hats.insert(
            "reviewer".to_string(),
            "You are the reviewer hat.\nReview code for quality.\n".to_string(),
        );
        let prompt_md = "# Objective\n\nHandle team management tasks.\n".to_string();
        (guardrails, hats, prompt_md)
    }

    #[test]
    fn meta_prompt_contains_role_identity() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("# Interactive Session — bob"),
            "Missing header"
        );
        assert!(
            result.contains("You are a member of the my-team team."),
            "Missing team identity"
        );
        assert!(result.contains("Your name is bob."), "Missing name");
        assert!(
            result.contains("Your role is called team-manager."),
            "Missing role"
        );
    }

    #[test]
    fn guardrails_included_with_numbering() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "alice",
            role_name: "architect",
            role_description: "Test role description",
            team_name: "dev-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(result.contains("## Guardrails"), "Missing Guardrails heading");
        assert!(
            result.contains("999. Always follow team invariants"),
            "Missing guardrail 999"
        );
        assert!(
            result.contains("1000. Use gh CLI for all GitHub operations"),
            "Missing guardrail 1000"
        );
    }

    #[test]
    fn prompt_md_content_in_role_context() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("## Role Context"),
            "Missing Role Context heading"
        );
        assert!(
            result.contains("Handle team management tasks"),
            "Missing PROMPT.md content"
        );
    }

    #[test]
    fn hatless_mode_includes_all_hats() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("### executor"),
            "Missing executor hat heading"
        );
        assert!(
            result.contains("### reviewer"),
            "Missing reviewer hat heading"
        );
        assert!(
            result.contains("Pick up tasks and execute them"),
            "Missing executor instructions"
        );
        assert!(
            result.contains("Review code for quality"),
            "Missing reviewer instructions"
        );
    }

    #[test]
    fn hat_specific_mode_includes_only_one_hat() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: Some("executor"),
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("Pick up tasks and execute them"),
            "Missing executor instructions"
        );
        assert!(
            !result.contains("Review code for quality"),
            "Reviewer instructions should not appear in executor-only mode"
        );
    }

    #[test]
    fn reference_materials_are_paths_not_inlined() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("## Reference: Operation Mode"),
            "Missing Reference heading"
        );
        assert!(
            result.contains("ralph-prompts/reference/"),
            "Missing reference path"
        );
        assert!(
            result.contains("These do not apply in interactive mode"),
            "Missing interactive mode note"
        );
    }

    #[test]
    fn output_is_well_formed_markdown() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);

        // Verify the meta-prompt's own heading starts with H1
        let lines: Vec<&str> = result.lines().collect();
        assert!(
            lines[0].starts_with("# Interactive Session"),
            "First line should be the H1 header"
        );

        // Verify the four structural H2 sections exist
        let h2_lines: Vec<&&str> = lines
            .iter()
            .filter(|l| l.starts_with("## ") && !l.starts_with("### "))
            .collect();
        let h2_texts: Vec<&str> = h2_lines.iter().map(|l| l.trim()).collect();
        assert!(h2_texts.contains(&"## Your Capabilities"));
        assert!(h2_texts.contains(&"## Guardrails"));
        assert!(h2_texts.contains(&"## Role Context"));
        assert!(h2_texts.contains(&"## Reference: Operation Mode"));
    }

    #[test]
    fn empty_guardrails_produces_empty_section() {
        let hats = BTreeMap::new();
        let params = MetaPromptParams {
            member_name: "x",
            role_name: "r",
            role_description: "",
            team_name: "t",
            guardrails: &[],
            hat_instructions: &hats,
            prompt_md_content: "",
            reference_dir: "ref/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        // Guardrails heading should exist even if empty
        assert!(result.contains("## Guardrails"));
        assert!(!result.contains("999."), "No numbered items when guardrails empty");
    }

    #[test]
    fn hat_specific_with_unknown_hat_produces_empty_capabilities() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: Some("nonexistent"),
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        // Should still produce valid output, just with empty capabilities
        assert!(result.contains("## Your Capabilities"));
        assert!(!result.contains("executor"));
        assert!(!result.contains("reviewer"));
    }

    #[test]
    fn meta_prompt_includes_role_description() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "superman",
            role_description: "All-in-one member -- PO, architect, dev, QE, SRE, content writer",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("All-in-one member -- PO, architect, dev, QE, SRE, content writer"),
            "Missing role description in identity section"
        );
    }

    #[test]
    fn meta_prompt_empty_role_description_no_blank_line() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "superman",
            role_description: "",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        // Role line should end with period and go straight to autonomy line
        assert!(
            result.contains("Your role is called superman.\nYou normally run autonomously"),
            "Empty role_description should not insert extra text after role name"
        );
    }

    #[test]
    fn skills_table_rendered_when_present() {
        let (guardrails, hats, prompt_md) = sample_params();
        let skills = vec![
            SkillInfo {
                name: "gh".to_string(),
                description: "Manages GitHub Projects v2 workflows".to_string(),
                load_command: "team/coding-agent/skills/gh/SKILL.md".to_string(),
            },
            SkillInfo {
                name: "status-workflow".to_string(),
                description: "Performs status transitions".to_string(),
                load_command: "team/coding-agent/skills/status-workflow/SKILL.md".to_string(),
            },
        ];
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &skills,
        };
        let result = build_meta_prompt(&params);

        // Verify Skills heading
        assert!(result.contains("## Skills"), "Missing Skills heading");
        // Verify table header
        assert!(
            result.contains("| Skill | Description | Load Command |"),
            "Missing table header"
        );
        // Verify both rows
        assert!(
            result.contains("| gh | Manages GitHub Projects v2 workflows | Read `team/coding-agent/skills/gh/SKILL.md` |"),
            "Missing gh skill row"
        );
        assert!(
            result.contains("| status-workflow | Performs status transitions | Read `team/coding-agent/skills/status-workflow/SKILL.md` |"),
            "Missing status-workflow skill row"
        );
        // Verify footer
        assert!(
            result.contains("To use a skill, read its SKILL.md file for full instructions."),
            "Missing skills footer"
        );

        // Verify Skills section appears between Capabilities and Guardrails
        let caps_pos = result.find("## Your Capabilities").unwrap();
        let skills_pos = result.find("## Skills").unwrap();
        let guard_pos = result.find("## Guardrails").unwrap();
        assert!(
            caps_pos < skills_pos && skills_pos < guard_pos,
            "Skills should appear between Capabilities and Guardrails"
        );

        // With skills, should have 5 H2 sections
        let lines: Vec<&str> = result.lines().collect();
        let h2_count = lines
            .iter()
            .filter(|l| l.starts_with("## ") && !l.starts_with("### "))
            .count();
        assert_eq!(h2_count, 5, "Should have 5 H2 sections when skills present");
    }

    #[test]
    fn skills_section_omitted_when_empty() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            !result.contains("## Skills"),
            "Skills heading should not appear when skills list is empty"
        );
    }

    #[test]
    fn interactive_mode_framing_present() {
        let (guardrails, hats, prompt_md) = sample_params();
        let params = MetaPromptParams {
            member_name: "bob",
            role_name: "team-manager",
            role_description: "Test role description",
            team_name: "my-team",
            guardrails: &guardrails,
            hat_instructions: &hats,
            prompt_md_content: &prompt_md,
            reference_dir: "ralph-prompts/reference/",
            hat: None,
            skills: &[],
        };
        let result = build_meta_prompt(&params);
        assert!(
            result.contains("Right now you are in an interactive session with the human (PO)"),
            "Missing interactive mode framing"
        );
        assert!(
            result.contains("You normally run autonomously inside Ralph Orchestrator"),
            "Missing autonomy context"
        );
    }
}
