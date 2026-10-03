use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use promptgen_core::json::{JsonValue, parse};

use super::{CodexExecutionError, io_error};

#[derive(Debug, PartialEq, Eq)]
enum CodexEvent {
    ThreadIdentity {
        id: String,
        contract: &'static str,
    },
    TurnCompleted,
    Failure(String),
    ForbiddenAction(String),
    /// An event type or item kind that is not on an explicit allowlist.
    UnknownAction(String),
    /// A known inert top-level event or turn item. It still counts as activity.
    BenignEvent(String),
    /// The one shell read Codex 0.144 performs to load the built-in image skill.
    SkillRead {
        id: String,
        command: String,
        completed: bool,
        succeeded: bool,
    },
}

/// Phase of a single Codex turn.
///
/// The published artifact is trusted only because the turn provably did nothing but
/// generate one image, so "what may still happen" is real state: an identity or an
/// action arriving after `turn.completed` means the log does not describe one closed
/// turn and the artifact cannot be attributed to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventPhase {
    AwaitingIdentity,
    Running,
    Completed,
}

impl CodexEvent {
    fn decode(value: JsonValue) -> Self {
        let JsonValue::Object(object) = value else {
            return Self::Failure("top-level event is not an object".to_owned());
        };
        let event_type = object.get("type").and_then(as_string).unwrap_or_default();
        match event_type {
            "thread.started" => thread_identity(&object, "thread_id", "thread.started/thread_id"),
            "session.created" => {
                thread_identity(&object, "session_id", "session.created/session_id")
            }
            "turn.completed" => Self::TurnCompleted,
            "turn.failed" | "error" => Self::Failure(event_message(&object)),
            "turn.started" => Self::BenignEvent(event_type.to_owned()),
            "item.started" | "item.updated" | "item.completed" => {
                let item = object.get("item").and_then(JsonValue::as_object);
                let kind = item
                    .and_then(|item| {
                        item.get("type")
                            .or_else(|| item.get("kind"))
                            .and_then(as_string)
                    })
                    .unwrap_or_default();
                match classify_item(kind) {
                    ItemKind::Benign => Self::BenignEvent(kind.to_owned()),
                    ItemKind::SkillRead => match item {
                        Some(item) => Self::SkillRead {
                            id: item
                                .get("id")
                                .and_then(as_string)
                                .unwrap_or_default()
                                .to_owned(),
                            command: item
                                .get("command")
                                .and_then(as_string)
                                .unwrap_or_default()
                                .to_owned(),
                            completed: event_type == "item.completed",
                            succeeded: item.get("status").and_then(as_string) == Some("completed")
                                && item.get("exit_code").and_then(JsonValue::as_i64) == Some(0),
                        },
                        None => Self::ForbiddenAction(
                            "command_execution missing item object".to_owned(),
                        ),
                    },
                    ItemKind::Failure => Self::Failure(format!("turn item reported {kind}")),
                    ItemKind::Forbidden => Self::ForbiddenAction(kind.to_owned()),
                    ItemKind::Unknown => Self::UnknownAction(format!("item kind {kind:?}")),
                }
            }
            _ => Self::UnknownAction(format!("top-level event type {event_type:?}")),
        }
    }
}

fn thread_identity(
    object: &BTreeMap<String, JsonValue>,
    field: &str,
    contract: &'static str,
) -> CodexEvent {
    match object
        .get(field)
        .and_then(as_string)
        .filter(|id| !id.is_empty())
    {
        Some(id) => CodexEvent::ThreadIdentity {
            id: id.to_owned(),
            contract,
        },
        None => CodexEvent::Failure(format!("event is missing non-empty {field}")),
    }
}

struct EventState {
    phase: EventPhase,
    thread_ids: BTreeSet<String>,
    contracts: BTreeSet<&'static str>,
    errors: Vec<String>,
    forbidden_actions: Vec<String>,
    unknown_actions: Vec<String>,
    skill_reads: BTreeMap<String, SkillReadState>,
    event_count: usize,
}

impl Default for EventState {
    fn default() -> Self {
        Self {
            phase: EventPhase::AwaitingIdentity,
            thread_ids: BTreeSet::new(),
            contracts: BTreeSet::new(),
            errors: Vec::new(),
            forbidden_actions: Vec::new(),
            unknown_actions: Vec::new(),
            skill_reads: BTreeMap::new(),
            event_count: 0,
        }
    }
}

impl EventState {
    fn apply(&mut self, event: CodexEvent) {
        self.event_count += 1;
        // Any activity after completion, including a known inert event, means the
        // log does not describe one closed turn.
        if self.phase == EventPhase::Completed && !matches!(event, CodexEvent::TurnCompleted) {
            self.errors
                .push(format!("event after turn completion: {event:?}"));
            return;
        }
        match event {
            CodexEvent::ThreadIdentity { id, contract } => {
                self.thread_ids.insert(id);
                self.contracts.insert(contract);
                self.phase = EventPhase::Running;
            }
            CodexEvent::TurnCompleted => self.phase = EventPhase::Completed,
            CodexEvent::Failure(message) => self.errors.push(message),
            CodexEvent::ForbiddenAction(kind) => self.forbidden_actions.push(kind),
            CodexEvent::UnknownAction(kind) => self.unknown_actions.push(kind),
            CodexEvent::SkillRead {
                id,
                command,
                completed,
                succeeded,
            } => self.apply_skill_read(id, command, completed, succeeded),
            CodexEvent::BenignEvent(_) => {}
        }
    }

    fn apply_skill_read(&mut self, id: String, command: String, completed: bool, succeeded: bool) {
        if id.is_empty() || command.is_empty() {
            self.forbidden_actions
                .push("command_execution missing id or command".to_owned());
            return;
        }
        let entry = self
            .skill_reads
            .entry(id)
            .or_insert_with(|| SkillReadState {
                command: command.clone(),
                completed: false,
                succeeded: false,
            });
        if entry.command != command {
            self.forbidden_actions
                .push("command_execution changed command for one item id".to_owned());
            return;
        }
        if completed {
            entry.completed = true;
            entry.succeeded = succeeded;
        }
    }

    fn finish(self, allowed_skill_path: Option<&Path>) -> Result<TurnSummary, CodexExecutionError> {
        if self.event_count == 0 {
            return Err(CodexExecutionError::new(
                "CODEX_EVENT_EMPTY",
                "Codex --json produced no events",
            ));
        }
        if !self.errors.is_empty() {
            return Err(CodexExecutionError::new(
                "CODEX_STREAM_ERROR",
                format!("event stream reported: {:?}", self.errors),
            ));
        }
        if !self.forbidden_actions.is_empty() {
            return Err(CodexExecutionError::new(
                "CODEX_FORBIDDEN_ACTION",
                format!(
                    "Codex turn attempted forbidden tool actions: {:?}",
                    self.forbidden_actions
                ),
            ));
        }
        for read in self.skill_reads.values() {
            if !allowed_skill_path.is_some_and(|path| is_allowed_skill_read(&read.command, path)) {
                return Err(CodexExecutionError::new(
                    "CODEX_FORBIDDEN_ACTION",
                    format!(
                        "Codex turn attempted unapproved shell command: {:?}",
                        read.command
                    ),
                ));
            }
            if !read.completed || !read.succeeded {
                return Err(CodexExecutionError::new(
                    "CODEX_STREAM_ERROR",
                    "approved image skill read did not complete successfully",
                ));
            }
        }
        if !self.unknown_actions.is_empty() {
            return Err(CodexExecutionError::new(
                "CODEX_UNKNOWN_ACTION",
                format!(
                    "Codex turn produced unrecognized events or item kinds: {:?}",
                    self.unknown_actions
                ),
            ));
        }
        if self.phase != EventPhase::Completed {
            return Err(CodexExecutionError::new(
                "CODEX_TURN_INCOMPLETE",
                "event stream has no turn.completed event",
            ));
        }
        if self.thread_ids.len() != 1 {
            return Err(CodexExecutionError::new(
                "CODEX_THREAD_ID",
                format!(
                    "expected exactly one thread id, found {:?}",
                    self.thread_ids
                ),
            ));
        }
        let thread_id = self.thread_ids.into_iter().next().expect("one id");
        let event_contract = self.contracts.into_iter().collect::<Vec<_>>().join("+");
        Ok(TurnSummary {
            thread_id,
            event_contract,
            image_skill_reads: self.skill_reads.len(),
        })
    }
}

struct SkillReadState {
    command: String,
    completed: bool,
    succeeded: bool,
}

/// What one accepted turn is known to have done.
#[derive(Debug)]
pub(super) struct TurnSummary {
    pub thread_id: String,
    pub event_contract: String,
    /// Distinct, successfully completed reads of the exact built-in image skill.
    pub image_skill_reads: usize,
}

pub(super) fn parse_and_validate(
    path: &Path,
    allowed_skill_path: Option<&Path>,
) -> Result<TurnSummary, CodexExecutionError> {
    let file = File::open(path).map_err(|error| io_error("open Codex event log", path, error))?;
    let mut state = EventState::default();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| io_error("read Codex event log", path, error))?;
        if line.trim().is_empty() {
            continue;
        }
        let value = parse(&line).map_err(|error| {
            CodexExecutionError::new(
                "CODEX_EVENT_JSON",
                format!("line {} is invalid JSON: {error}", index + 1),
            )
        })?;
        state.apply(CodexEvent::decode(value));
    }
    state.finish(allowed_skill_path)
}

enum ItemKind {
    Benign,
    SkillRead,
    Failure,
    Forbidden,
    Unknown,
}

/// Classifies a turn item.
///
/// A deny list alone admits every kind that has not been thought of yet: one new
/// tool item type in a future Codex release would pass unnoticed and still publish
/// an artifact. Only the kinds known to be inert are accepted; anything else fails
/// the run and names itself in the error so the list can be revisited deliberately.
fn classify_item(kind: &str) -> ItemKind {
    match kind {
        "agent_message"
        | "assistant_message"
        | "user_message"
        | "system_message"
        | "reasoning"
        | "todo_list"
        | "image_generation"
        | "image_generation_call" => ItemKind::Benign,
        // An error item is a reported failure, not inert content.
        "error" => ItemKind::Failure,
        // A command execution is decoded with its identity and command text, then
        // accepted only when it is the exact built-in image-skill read. Read-only
        // sandbox mode is defense in depth, not the authorization decision.
        "command_execution" => ItemKind::SkillRead,
        "file_change" | "mcp_tool_call" | "web_search" | "dynamic_tool_call"
        | "collab_tool_call" => ItemKind::Forbidden,
        _ => ItemKind::Unknown,
    }
}

fn is_allowed_skill_read(command: &str, allowed_skill_path: &Path) -> bool {
    let path = allowed_skill_path.display().to_string();
    if path
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/._-".contains(character))
        && command == format!("/bin/zsh -lc 'cat {path}'")
    {
        return true;
    }
    let unquoted = format!("/bin/zsh -lc \"sed -n '1,240p' {path}\"");
    if command == unquoted {
        return true;
    }
    !path.contains('\'') && command == format!("/bin/zsh -lc \"sed -n '1,240p' '{path}'\"")
}

fn event_message(object: &BTreeMap<String, JsonValue>) -> String {
    object
        .get("message")
        .and_then(as_string)
        .or_else(|| {
            object.get("error").and_then(|value| match value {
                JsonValue::String(value) => Some(value.as_str()),
                JsonValue::Object(error) => error.get("message").and_then(as_string),
                _ => None,
            })
        })
        .unwrap_or("unspecified Codex error")
        .to_owned()
}

pub(crate) fn as_string(value: &JsonValue) -> Option<&str> {
    match value {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SKILL_PATH: &str = "/Users/test/.codex/skills/.system/imagegen/SKILL.md";

    fn apply_json(state: &mut EventState, source: &str) {
        state.apply(CodexEvent::decode(parse(source).unwrap()));
    }

    fn finish(state: EventState) -> Result<TurnSummary, CodexExecutionError> {
        state.finish(Some(Path::new(SKILL_PATH)))
    }

    #[test]
    fn completed_single_thread_is_valid() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.started"}"#);
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        let turn = finish(state).unwrap();
        assert_eq!(turn.thread_id, "thread-1");
        assert_eq!(turn.event_contract, "thread.started/thread_id");
        assert_eq!(turn.image_skill_reads, 0);
    }

    #[test]
    fn exact_completed_skill_read_is_permitted_and_deduplicated() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.started","item":{"id":"item-1","type":"command_execution","command":"/bin/zsh -lc \"sed -n '1,240p' /Users/test/.codex/skills/.system/imagegen/SKILL.md\"","exit_code":null,"status":"in_progress"}}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"id":"item-1","type":"command_execution","command":"/bin/zsh -lc \"sed -n '1,240p' /Users/test/.codex/skills/.system/imagegen/SKILL.md\"","exit_code":0,"status":"completed"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        let turn = finish(state).unwrap();
        assert_eq!(turn.image_skill_reads, 1);
    }

    #[test]
    fn observed_cat_skill_read_is_admitted_but_extra_actions_are_rejected() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"id":"item-cat","type":"command_execution","command":"/bin/zsh -lc 'cat /Users/test/.codex/skills/.system/imagegen/SKILL.md'","exit_code":0,"status":"completed"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert_eq!(finish(state).unwrap().image_skill_reads, 1);
        for command in [
            format!("/bin/zsh -lc 'cat {SKILL_PATH}; printf owned'"),
            "/bin/zsh -lc 'cat /Users/test/.codex/auth.json'".to_owned(),
            format!("/bin/zsh -lc 'cat {SKILL_PATH} > output'"),
        ] {
            assert!(!is_allowed_skill_read(&command, Path::new(SKILL_PATH)));
        }
        assert!(!is_allowed_skill_read(
            "/bin/zsh -lc 'cat /tmp/$(touch-owned)/SKILL.md'",
            Path::new("/tmp/$(touch-owned)/SKILL.md")
        ));
    }

    #[test]
    fn arbitrary_shell_command_is_forbidden_even_in_read_only_mode() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"id":"item-1","type":"command_execution","command":"/bin/zsh -lc \"printf owned > marker\"","exit_code":0,"status":"completed"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        let error = finish(state).unwrap_err();
        assert_eq!(error.code, "CODEX_FORBIDDEN_ACTION");
        assert!(error.message.contains("printf owned"));
    }

    #[test]
    fn command_without_identity_or_completion_fails_closed() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"type":"command_execution"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert_eq!(finish(state).unwrap_err().code, "CODEX_FORBIDDEN_ACTION");
    }

    #[test]
    fn write_and_network_tools_remain_forbidden() {
        for kind in [
            "file_change",
            "mcp_tool_call",
            "web_search",
            "dynamic_tool_call",
            "collab_tool_call",
        ] {
            let mut state = EventState::default();
            apply_json(
                &mut state,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            );
            apply_json(
                &mut state,
                &format!(r#"{{"type":"item.completed","item":{{"type":"{kind}"}}}}"#),
            );
            apply_json(&mut state, r#"{"type":"turn.completed"}"#);
            assert_eq!(
                finish(state).unwrap_err().code,
                "CODEX_FORBIDDEN_ACTION",
                "{kind}"
            );
        }
    }

    #[test]
    fn transition_rejects_forbidden_tool_action() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"type":"file_change"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert_eq!(finish(state).unwrap_err().code, "CODEX_FORBIDDEN_ACTION");
    }

    /// A deny list would silently accept any item kind a future Codex release adds.
    #[test]
    fn transition_rejects_an_unrecognized_item_kind() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"type":"shell_call_v2"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        let error = finish(state).unwrap_err();
        assert_eq!(error.code, "CODEX_UNKNOWN_ACTION");
        assert!(error.message.contains("shell_call_v2"));
    }

    #[test]
    fn top_level_tool_and_web_events_cannot_bypass_item_classification() {
        for event_type in ["tool.completed", "web_search"] {
            let mut state = EventState::default();
            apply_json(
                &mut state,
                r#"{"type":"thread.started","thread_id":"thread-1"}"#,
            );
            apply_json(
                &mut state,
                &format!(r#"{{"type":"{event_type}","status":"completed"}}"#),
            );
            apply_json(&mut state, r#"{"type":"turn.completed"}"#);
            let error = finish(state).unwrap_err();
            assert_eq!(error.code, "CODEX_UNKNOWN_ACTION", "{event_type}");
            assert!(error.message.contains(event_type), "{event_type}");
        }
    }

    #[test]
    fn missing_top_level_event_type_fails_closed() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(&mut state, r#"{"message":"not typed"}"#);
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert_eq!(finish(state).unwrap_err().code, "CODEX_UNKNOWN_ACTION");
    }

    #[test]
    fn benign_item_kinds_do_not_fail_the_turn() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        for kind in [
            "agent_message",
            "reasoning",
            "image_generation",
            "todo_list",
        ] {
            apply_json(
                &mut state,
                &format!(r#"{{"type":"item.completed","item":{{"type":"{kind}"}}}}"#),
            );
        }
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert!(finish(state).is_ok());
    }

    /// An `error` item is a reported failure even though it carries no tool action.
    #[test]
    fn transition_rejects_an_error_item() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"type":"error"}}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert_eq!(finish(state).unwrap_err().code, "CODEX_STREAM_ERROR");
    }

    /// Even an inert item means the turn was not closed where it claimed to be.
    #[test]
    fn transition_rejects_a_benign_item_after_completion() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        apply_json(
            &mut state,
            r#"{"type":"item.completed","item":{"type":"image_generation"}}"#,
        );
        let error = finish(state).unwrap_err();
        assert_eq!(error.code, "CODEX_STREAM_ERROR");
        assert!(error.message.contains("after turn completion"));
    }

    /// The artifact is attributed to one closed turn, so activity after the turn
    /// completed means the log no longer describes what produced the image.
    #[test]
    fn transition_rejects_activity_after_completion() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-2"}"#,
        );
        let error = finish(state).unwrap_err();
        assert_eq!(error.code, "CODEX_STREAM_ERROR");
        assert!(error.message.contains("after turn completion"));
    }

    #[test]
    fn transition_rejects_missing_identity_field() {
        let mut state = EventState::default();
        apply_json(&mut state, r#"{"type":"thread.started"}"#);
        apply_json(&mut state, r#"{"type":"turn.completed"}"#);
        assert_eq!(finish(state).unwrap_err().code, "CODEX_STREAM_ERROR");
    }

    #[test]
    fn transition_requires_terminal_completion() {
        let mut state = EventState::default();
        apply_json(
            &mut state,
            r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        );
        assert_eq!(finish(state).unwrap_err().code, "CODEX_TURN_INCOMPLETE");
    }
}
