// A fake Anthropic Messages API server. Claude Code (pointed here via
// ANTHROPIC_BASE_URL) drives a real agentic loop against scripted assistant
// turns — so the tests are deterministic and cost nothing, while every tool
// surface, hook dispatch, and the binary all run for real.
//
// The server is ALSO the observation point: each request body carries the full
// message history, so the `tool_result` Claude Code sends back IS exactly what
// the model received after the PostToolUse hook ran. No breadcrumbs needed.
//
// Turn selection is driven by the conversation state (count of tool_results seen
// + whether the request carries a `tools` array), never a blind request counter,
// so an auxiliary request (e.g. title generation) can't desync the scenario.

import http from "node:http";

import { nonStreamBody, streamTurn } from "./sse.mjs";
import { estimateTokens } from "./lib.mjs";

/** Count tool_result content blocks across every message in a request body. */
function toolResultCount(body) {
  let n = 0;
  for (const m of body?.messages ?? []) {
    if (!Array.isArray(m.content)) continue;
    for (const b of m.content) if (b?.type === "tool_result") n += 1;
  }
  return n;
}

/** A request belongs to the agent loop only if it advertises tools. */
function isAgentLoop(body) {
  return Array.isArray(body?.tools) && body.tools.length > 0;
}

/** Flatten a tool_result `content` (string | block array) to plain text. */
function resultText(content) {
  if (typeof content === "string") return content;
  if (Array.isArray(content)) {
    return content.map((b) => (typeof b === "string" ? b : (b?.text ?? ""))).join("");
  }
  return content?.text ?? "";
}

export class MockAnthropic {
  /**
   * @param {{toolTurns?:Array, finalText?:string, responder?:Function}} scenario
   *   `toolTurns` are emitted one per tool_result already seen; once exhausted a
   *   final `end_turn` text reply closes the loop. `responder(body, ctx)` fully
   *   overrides selection for advanced cases.
   */
  constructor(scenario = {}) {
    this.toolTurns = scenario.toolTurns ?? [];
    this.finalText = scenario.finalText ?? "done";
    this.responder = scenario.responder ?? null;
    this.requests = [];
    this.unknownPaths = [];
    this.calls = 0;
    this.server = null;
  }

  get port() {
    return this.server.address().port;
  }

  get baseUrl() {
    return `http://127.0.0.1:${this.port}`;
  }

  async start() {
    this.server = http.createServer((req, res) => this.#handle(req, res));
    await new Promise((resolve) => this.server.listen(0, "127.0.0.1", resolve));
    return this.baseUrl;
  }

  async stop() {
    if (this.server) await new Promise((resolve) => this.server.close(resolve));
  }

  #nextTurn(body) {
    if (this.responder) return this.responder(body, { calls: this.calls });
    if (!isAgentLoop(body)) return { text: "ack", stop: "end_turn" };
    const k = toolResultCount(body);
    if (k < this.toolTurns.length) return this.toolTurns[k];
    return { text: this.finalText, stop: "end_turn" };
  }

  #handle(req, res) {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => {
      const raw = Buffer.concat(chunks).toString("utf8");
      let body = null;
      try {
        body = raw ? JSON.parse(raw) : null;
      } catch {
        body = raw;
      }
      this.requests.push({ method: req.method, url: req.url, headers: req.headers, body });
      const path = req.url.split("?")[0];
      if (req.method === "POST" && path === "/v1/messages") return this.#messages(res, body);
      if (path.endsWith("/count_tokens")) {
        const n = estimateTokens(JSON.stringify(body?.messages ?? body ?? ""));
        return this.#json(res, 200, { input_tokens: n });
      }
      // Anything else Claude Code might ping at startup: answer benignly + log it.
      this.unknownPaths.push(path);
      return this.#json(res, 200, {});
    });
  }

  #messages(res, body) {
    const idx = this.calls;
    this.calls += 1;
    const model = body?.model ?? "claude-mock";
    const turn = this.#nextTurn(body);
    if (body?.stream === true) {
      res.writeHead(200, {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      });
      for (const ev of streamTurn(turn, { model, index: idx })) res.write(ev);
      return res.end();
    }
    return this.#json(res, 200, nonStreamBody(turn, { model, index: idx }));
  }

  #json(res, code, obj) {
    const s = JSON.stringify(obj);
    res.writeHead(code, {
      "Content-Type": "application/json",
      "Content-Length": Buffer.byteLength(s),
    });
    res.end(s);
  }

  // --- assertion helpers (read after the run) ---

  /** Every tool_result the model received, keyed by `tool_use_id` → text. */
  toolResults() {
    const out = {};
    for (const r of this.requests) {
      for (const m of r.body?.messages ?? []) {
        if (m.role !== "user" || !Array.isArray(m.content)) continue;
        for (const b of m.content) {
          if (b?.type === "tool_result") out[b.tool_use_id] = resultText(b.content);
        }
      }
    }
    return out;
  }

  /** How many `/v1/messages` requests carried a `tools` array (agent turns). */
  agentTurns() {
    return this.requests.filter(
      (r) => r.url.split("?")[0] === "/v1/messages" && isAgentLoop(r.body),
    ).length;
  }
}
