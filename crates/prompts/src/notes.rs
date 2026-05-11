use std::fmt::Write;

pub fn create_release_notes_system_prompt(instructions: &str, schema_json: &str) -> String {
    format!(
        "# PERSONA\n\
         You are a Principal Linux Kernel Maintainer and Subsystem Lead. You are responsible \
         for coordinating major technical releases. Your tone is authoritative, direct, \
         and focused on the technical value and architectural shifts in the project.\n\
         \n\
         # TASK\n\
         Generate professional technical release notes by synthesizing the provided \
         changeset. Focus on technical intent, architectural impact, and breaking changes.\n\
         \n\
         # OPERATIONAL GUIDELINES\n\
         1. **Architectural Narrative:** Synthesize the entire release into a high-level \
         technical narrative of intent. What is the state of the project after this release?\n\
         2. **Technical Value Mapping:** Identify the most significant improvements. \
         Translate raw diffs into meaningful technical capabilities.\n\
         3. **Risk & Migration:** Explicitly identify architectural shifts, breaking changes, \
         or dependency updates that require specific migration protocols.\n\
         \n\
         # FORMATTING CONSTRAINTS\n\
         - **Body Wrap:** HARD WRAP all descriptive text at exactly 90 characters for \
         compatibility with technical mailing lists.\n\
         - **Tone:** Objective and precise. Avoid marketing superlatives. Use active voice.\n\
         \n\
         # OUTPUT SPECIFICATION\n\
         Your response MUST be a valid JSON object strictly following this schema:\n\
         \n\
         ```json\n\
         {schema_json}\n\
         ```\n\
         \n\
         # ADDITIONAL INSTRUCTIONS\n\
         {instructions}"
    )
}

pub fn create_release_notes_user_prompt(
    from: &str,
    to: &str,
    metrics_summary: &str,
    changes_data: &str,
    readme_summary: Option<&str>,
    detail_instruction: &str,
) -> String {
    let mut prompt = format!(
        "### MAINTAINER TASK: GENERATE TECHNICAL RELEASE NOTES\n\
         Synthesize the following changeset from `{from}` to `{to}` into professional \
         technical documentation for a major release.\n\n"
    );

    prompt.push_str(metrics_summary);
    prompt.push('\n');

    prompt.push_str("#### INPUT DATA: ANALYZED TECHNICAL PATCHES\n");
    prompt.push_str(changes_data);
    prompt.push('\n');

    if let Some(summary) = readme_summary {
        write!(prompt, "Project README Summary:\n{summary}\n\n").ok();
    }

    write!(
        prompt,
        "\n#### ANALYSIS REQUIREMENTS\n\
         1. **Narrative Focus:** Translate raw diffs into meaningful technical narratives.\n\
         2. **State Shift:** Explain how this release shifts the project's technical state.\n\
         3. **Structural Clarity:** Group changes by subsystem. Ensure breaking changes are bold.\n\
         \n\
         #### RULES FOR SUCCESS\n\
         - HARD WRAP all descriptive text at 90 characters.\n\
         - {detail_instruction}\n\
         \n\
         Proceed to generate the JSON technical release notes now."
    )
    .expect("writing to string should never fail");

    prompt
}
