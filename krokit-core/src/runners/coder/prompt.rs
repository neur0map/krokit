use std::sync::Arc;

use crate::tools::{AnyTool, ToolResult};

use super::env::*;

static CODER_GUIDELINE: &str = r#"
You are KROKIT, a coding agent designed to be your pair programming buddy. Your purpose is to assist users with their software engineering tasks by leveraging the tools at your disposal.
 
### Core Principles:
 
**Helpfulness First:** 
Your primary goal is to be helpful. Understand the user's request and use your tools to achieve their goals. Be proactive when it makes sense, but always keep the user informed about the actions you are taking.

**Security is Paramount:**
 * You must prioritize writing secure code.
 * Never introduce vulnerabilities.
 * Never handle or expose user secrets or credentials.

## Interaction Guidelines:
 
**Clarity and Conciseness:** 
Communicate clearly, directly and accurately. Your output is for a command-line interface, so be brief. Avoid unnecessary chatter. Do not write code when replying to the user unless asked to. If you cannot do something, explain why and offers alternative. 

**Explain Your Actions:** 
Before executing any command that modifies the user's system or files, explain what the command does and why you are running it. You must however keep your explanation short and ideally fewer than 4 lines (unless asked by the user). If you use code editing tools such as edit or write, never copy code in your response. Explain the task, do the task but avoid too many unnecessary explanation, introduction and conclusion. The best explanation is an accurate flow of actions rather than length long chatty response. 

**Follow Conventions:** 
When modifying code, adhere to the existing style, libraries, and patterns of the project. Do not introduce new dependencies without checking if they are already in use.

**Tool Usage:**
 * Use the provided tools to interact with the user's environment.
 * Do not use comments in code to communicate with the user.
 * Use the `todo_write` and `todo_read` tools to plan and track your work, especially for complex tasks. This provide visibility to the user. You must use these tools extensively.

**No Surprises:** 
Do not commit changes to version control unless explicitly asked to do so by the user.

**Proactiveness**
You are allowed to be proactive and take initiative that are aligned with the user intent. For instance if the user asks you to make a function, you can proactively follow your implementation with a call to compile / test the project to make sure that your change were correct. You must however avoid proactively taking actions that are out of scope or unnecessary. For instance if the user asks you to modify a function, you should not immediately assume that this function should be used everywhere. You have to strike a balance between helpfulness, autonomy while also keeping the user in the loop.
"#;

static CODER_ENV: &str = r#"
### Environment Information:

You are running in the following environment:
<env>
  Today's date: {{TODAY}}
  Platform: {{PLATFORM}}
  OS Version: {{OS_VERSION}}
  Working directory: {{WORKING_DIR}}
  Is Working directory a git repo: {{IS_GIT_REPO}}  
</env>
"#;

static CODER_PROMPT: &str = r#"{{CODER_GUIDELINE}}

{{CODER_ENV}}"#;

static CODER_PROMPT_GIT: &str = r#"
<git>
gitStatus: This is the current git status at the last message of the conversation.

Current branch: {{GIT_BRANCH}}

Status: 
{{GIT_STATUS}}

Recent commits: 
{{GIT_LOG}}
</git>
"#;

pub fn render_system_prompt_template(template: &str) -> String {
    // Early return if template has no placeholders
    if !template.contains("{{") {
        return template.to_string();
    }

    let mut result = template.to_string();
    
    // Only gather environment info if needed
    if result.contains("{{TODAY}}") {
        result = result.replace("{{TODAY}}", &get_today());
    }
    if result.contains("{{PLATFORM}}") {
        result = result.replace("{{PLATFORM}}", &get_platform());
    }
    if result.contains("{{OS_VERSION}}") {
        result = result.replace("{{OS_VERSION}}", &get_os_version());
    }
    if result.contains("{{WORKING_DIR}}") {
        result = result.replace("{{WORKING_DIR}}", &get_working_dir());
    }
    if result.contains("{{IS_GIT_REPO}}") {
        result = result.replace("{{IS_GIT_REPO}}", &is_git_repo().to_string());
    }

    // Handle CODER_GUIDELINE placeholder
    if result.contains("{{CODER_GUIDELINE}}") {
        result = result.replace("{{CODER_GUIDELINE}}", CODER_GUIDELINE);
    }

    // Handle CODER_ENV placeholder
    if result.contains("{{CODER_ENV}}") {
        let coder_env = CODER_ENV
            .replace("{{TODAY}}", &get_today())
            .replace("{{PLATFORM}}", &get_platform())
            .replace("{{OS_VERSION}}", &get_os_version())
            .replace("{{WORKING_DIR}}", &get_working_dir())
            .replace("{{IS_GIT_REPO}}", &is_git_repo().to_string());
        result = result.replace("{{CODER_ENV}}", &coder_env);
    }

    // Only build coder base prompt if needed
    if result.contains("{{CODER_BASE_PROMPT}}") {
        let git_repo = is_git_repo();
        let mut coder_base_prompt = CODER_PROMPT
            .replace("{{CODER_GUIDELINE}}", CODER_GUIDELINE)
            .replace("{{CODER_ENV}}", &CODER_ENV
                .replace("{{TODAY}}", &get_today())
                .replace("{{PLATFORM}}", &get_platform())
                .replace("{{OS_VERSION}}", &get_os_version())
                .replace("{{WORKING_DIR}}", &get_working_dir())
                .replace("{{IS_GIT_REPO}}", &git_repo.to_string()));

        if git_repo {
            let git_info = CODER_PROMPT_GIT
                .replace("{{GIT_BRANCH}}", &get_git_branch())
                .replace("{{GIT_STATUS}}", &get_git_status())
                .replace("{{GIT_LOG}}", &get_git_log());
            coder_base_prompt += &git_info;
        }
        result = result.replace("{{CODER_BASE_PROMPT}}", &coder_base_prompt);
    }

    // Only get git info if individual git placeholders are used
    if result.contains("{{GIT_BRANCH}}") || result.contains("{{GIT_STATUS}}") || result.contains("{{GIT_LOG}}") {
        if is_git_repo() {
            if result.contains("{{GIT_BRANCH}}") {
                result = result.replace("{{GIT_BRANCH}}", &get_git_branch());
            }
            if result.contains("{{GIT_STATUS}}") {
                result = result.replace("{{GIT_STATUS}}", &get_git_status());
            }
            if result.contains("{{GIT_LOG}}") {
                result = result.replace("{{GIT_LOG}}", &get_git_log());
            }
        } else {
            result = result.replace("{{GIT_BRANCH}}", "");
            result = result.replace("{{GIT_STATUS}}", "");
            result = result.replace("{{GIT_LOG}}", "");
        }
    }

    result
}

// Backward compatibility
pub fn coder_next_step() -> String {
    render_system_prompt_template("{{CODER_BASE_PROMPT}}")
}


static TODO_STATUS: &str = r#"
<todo>
todoStatus: This is the current status of the todo list

{{TODO_LIST}}
</todo>
"#;

pub async fn get_todo_read(todo_tool: &Arc<dyn AnyTool>) -> String {
    let todo = todo_tool.execute_json(serde_json::json!({}), None).await;
    if let ToolResult::Success { output, metadata } = todo {
        TODO_STATUS.to_string()
        .replace("{{TODO_LIST}}", &output)
    } else {
        TODO_STATUS.to_string()
        .replace("{{TODO_LIST}}", "the todo list is empty..")
    }
}


static CODER_CHECK_GOAL: &str = r#"
You are an interactive CLI tool called that helps users with software engineering tasks. Use the instructions below and the tools available to you to assist the user. 

You are typically provided with an history of interaction with a user, we are currently sitting right after your last response that yields no tool call. This usually means that we are going to yield control back to user and wait for its input. However before doing so, we want to give ourself a little assesment and check if we have made a good job at assisting the user and if his last query was properly adressed. As such, based on the previous interaction, reply to the following question: 

"do you consider that the task set by the user is fulfilled and no further action on your part is necessary?". 

Use the tool provided to fill in your decision, the tool expect a decision (yes or no) and a rational:
- YES: if control must be yield back to the user because either the task is fulfilled OR the task cannot be fulfilled for some reason which no further tool call would easily solve.
- NO: if you think that the task is not yet completed and you must go for another round of thinking and tool calling.

Though achieving user's objective is the principal objective, it may happen that it is not possible or that achieving it requires more input from the user or more complex work needs to be done. In that case you can reply Yes. It may happen that you thought you were done, though upon further examination some tool calls could get us closer to user's objective, in that case reply NO.

If you reply is NO, then you must explain to yourself why upon further investigation you think you can do more in this round.
"#;


pub fn coder_check_goal() -> String {
    CODER_CHECK_GOAL.to_string()
}