// Anthropic Messages API response encoders. Turns a scripted "turn" (a text
// reply and/or a single tool_use) into either the streaming SSE event sequence
// Claude Code consumes when it sends `"stream": true`, or a one-shot JSON body.
//
// The streaming event order mirrors the real API: message_start → (text block)
// → (tool_use block) → message_delta(stop_reason) → message_stop. If Claude Code
// pins a newer schema, the mock's request log shows exactly what it rejected.

/** Frame one SSE event: `event:`/`data:` lines + the blank-line terminator. */
function frame(event, data) {
  return `event: ${event}\ndata: ${JSON.stringify(data)}\n\n`;
}

/**
 * The full SSE event stream for one assistant `turn`.
 * @param {{text?:string, toolUse?:{id:string,name:string,input:object}, stop?:string, inputTokens?:number, outputTokens?:number}} turn
 * @param {{model:string, index:number}} ctx
 */
export function* streamTurn(turn, { model, index }) {
  const id = `msg_mock_${index}`;
  const usage = { input_tokens: turn.inputTokens ?? 8, output_tokens: 1 };

  yield frame("message_start", {
    type: "message_start",
    message: {
      id,
      type: "message",
      role: "assistant",
      model,
      content: [],
      stop_reason: null,
      stop_sequence: null,
      usage,
    },
  });
  yield frame("ping", { type: "ping" });

  let block = 0;
  if (turn.text) {
    yield frame("content_block_start", {
      type: "content_block_start",
      index: block,
      content_block: { type: "text", text: "" },
    });
    yield frame("content_block_delta", {
      type: "content_block_delta",
      index: block,
      delta: { type: "text_delta", text: turn.text },
    });
    yield frame("content_block_stop", { type: "content_block_stop", index: block });
    block += 1;
  }

  if (turn.toolUse) {
    const t = turn.toolUse;
    yield frame("content_block_start", {
      type: "content_block_start",
      index: block,
      content_block: { type: "tool_use", id: t.id, name: t.name, input: {} },
    });
    yield frame("content_block_delta", {
      type: "content_block_delta",
      index: block,
      delta: { type: "input_json_delta", partial_json: JSON.stringify(t.input ?? {}) },
    });
    yield frame("content_block_stop", { type: "content_block_stop", index: block });
    block += 1;
  }

  const stop = turn.stop ?? (turn.toolUse ? "tool_use" : "end_turn");
  yield frame("message_delta", {
    type: "message_delta",
    delta: { stop_reason: stop, stop_sequence: null },
    usage: { output_tokens: turn.outputTokens ?? 12 },
  });
  yield frame("message_stop", { type: "message_stop" });
}

/** The non-streaming Messages API body for one assistant `turn`. */
export function nonStreamBody(turn, { model, index }) {
  const content = [];
  if (turn.text) content.push({ type: "text", text: turn.text });
  if (turn.toolUse) {
    content.push({
      type: "tool_use",
      id: turn.toolUse.id,
      name: turn.toolUse.name,
      input: turn.toolUse.input ?? {},
    });
  }
  return {
    id: `msg_mock_${index}`,
    type: "message",
    role: "assistant",
    model,
    content,
    stop_reason: turn.stop ?? (turn.toolUse ? "tool_use" : "end_turn"),
    stop_sequence: null,
    usage: { input_tokens: turn.inputTokens ?? 8, output_tokens: turn.outputTokens ?? 12 },
  };
}
