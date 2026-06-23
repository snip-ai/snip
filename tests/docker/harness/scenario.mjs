// Scenario builders: scripted assistant turns the mock server replays. A tool
// turn carries a deterministic `tool_use` id so a test can look up exactly what
// the model received back for that call via `MockAnthropic#toolResults()`.

let counter = 0;

/** Reset the tool-use id counter (call at the top of a test for stable ids). */
export function resetIds() {
  counter = 0;
}

/**
 * An assistant turn that calls one tool.
 * @param {string} name  real Claude Code tool name (Read|Grep|Glob|Bash|Edit|Write)
 * @param {object} input the tool_input the model "chose"
 * @param {string} [text] optional preamble text before the tool call
 */
export function toolTurn(name, input, text) {
  counter += 1;
  return {
    text,
    toolUse: { id: `toolu_mock_${counter}`, name, input },
    stop: "tool_use",
  };
}

/** A final assistant turn that ends the conversation with plain text. */
export function textTurn(text) {
  return { text, stop: "end_turn" };
}
