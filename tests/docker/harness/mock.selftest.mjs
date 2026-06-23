// Self-test for the mock server's protocol, with NO Claude Code involved: we
// hand-play the two requests a real agent loop would send (turn 1 carries tools
// and no tool_result → expect a tool_use; turn 2 carries the tool_result →
// expect a final end_turn) and assert the SSE parses and the observation helpers
// read back what the "model" saw. Run: `node tests/docker/harness/mock.selftest.mjs`.

import assert from "node:assert/strict";

import { MockAnthropic } from "./mock-anthropic.mjs";
import { resetIds, toolTurn } from "./scenario.mjs";

/** Minimal SSE parser: returns the list of parsed `data:` JSON objects. */
async function readSse(res) {
  const text = await res.text();
  const events = [];
  for (const block of text.split("\n\n")) {
    const line = block.split("\n").find((l) => l.startsWith("data:"));
    if (line) events.push(JSON.parse(line.slice(5).trim()));
  }
  return events;
}

async function post(baseUrl, body) {
  return fetch(`${baseUrl}/v1/messages`, {
    method: "POST",
    headers: { "content-type": "application/json", "x-api-key": "k" },
    body: JSON.stringify(body),
  });
}

async function main() {
  resetIds();
  const read = toolTurn("Read", { file_path: "/work/x.rs" });
  const mock = new MockAnthropic({ toolTurns: [read], finalText: "all done" });
  await mock.start();

  // Turn 1: tools advertised, no tool_result yet → expect a streamed tool_use.
  const r1 = await post(mock.baseUrl, {
    model: "claude-mock",
    stream: true,
    tools: [{ name: "Read" }],
    messages: [{ role: "user", content: "read it" }],
  });
  const ev1 = await readSse(r1);
  const types1 = ev1.map((e) => e.type);
  assert.ok(types1.includes("message_start"), "turn1 has message_start");
  const tu = ev1.find((e) => e.type === "content_block_start" && e.content_block?.type === "tool_use");
  assert.ok(tu, "turn1 emits a tool_use block");
  assert.equal(tu.content_block.id, read.toolUse.id, "tool_use id matches scenario");
  const delta = ev1.find((e) => e.delta?.type === "input_json_delta");
  assert.equal(JSON.parse(delta.delta.partial_json).file_path, "/work/x.rs", "tool input streamed");
  const stop1 = ev1.find((e) => e.type === "message_delta");
  assert.equal(stop1.delta.stop_reason, "tool_use", "turn1 stop_reason=tool_use");

  // Turn 2: the agent returns the tool_result → expect a final end_turn text.
  const r2 = await post(mock.baseUrl, {
    model: "claude-mock",
    stream: true,
    tools: [{ name: "Read" }],
    messages: [
      { role: "user", content: "read it" },
      { role: "assistant", content: [{ type: "tool_use", id: read.toolUse.id, name: "Read", input: {} }] },
      {
        role: "user",
        content: [{ type: "tool_result", tool_use_id: read.toolUse.id, content: "[snip: read | rust] fn main(){}" }],
      },
    ],
  });
  const ev2 = await readSse(r2);
  const txt = ev2.find((e) => e.delta?.type === "text_delta");
  assert.equal(txt.delta.text, "all done", "turn2 emits the final text");
  const stop2 = ev2.find((e) => e.type === "message_delta");
  assert.equal(stop2.delta.stop_reason, "end_turn", "turn2 stop_reason=end_turn");

  // The observation helper reads back exactly what the "model" received.
  const seen = mock.toolResults()[read.toolUse.id];
  assert.match(seen, /\[snip: read \| rust\]/, "toolResults() recovers the model-visible content");
  assert.equal(mock.agentTurns(), 2, "counted two agent-loop turns");

  // Non-streaming path also returns a well-formed body.
  const r3 = await fetch(`${mock.baseUrl}/v1/messages`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ model: "m", messages: [{ role: "user", content: "hi" }] }),
  });
  const j3 = await r3.json();
  assert.equal(j3.type, "message", "non-stream body shape");

  // count_tokens stub answers.
  const r4 = await fetch(`${mock.baseUrl}/v1/messages/count_tokens`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ messages: [{ role: "user", content: "hello" }] }),
  });
  const j4 = await r4.json();
  assert.equal(typeof j4.input_tokens, "number", "count_tokens stub answers a number");

  await mock.stop();
  console.log("mock.selftest: OK");
}

main().catch((e) => {
  console.error("mock.selftest: FAIL");
  console.error(e);
  process.exit(1);
});
