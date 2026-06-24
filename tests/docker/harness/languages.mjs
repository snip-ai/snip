// Every language snip's `read` optimizer supports, as a generated commented
// fixture. The header name is the registry's LanguageSpec.name (a direct
// pass-through), so a per-language test can assert "[snip: read | <name>". Each
// fixture buries SNIP_SECRET_MARKER in a comment block.
//
// The block must be big enough that stripping it clears the read optimizer's
// no-inflation guard. That guard weighs the rewrite against a FIXED two-line
// cost: the `[snip: read | …]` header plus the multi-sentence guidance banner
// (which embeds the absolute binary path, ~60+ tokens). A handful of short
// comment lines does NOT beat that banner, so the fixtures use many long lines
// (~450 tokens of comment) — comfortably past the guard with margin to spare,
// regardless of how long the install path is. A passing test then proves: the
// language was detected, comments were stripped, and the model saw the header.

const MARKER = "SNIP_SECRET_MARKER";
const COMMENT_LINES = 24;
const FILLER =
  "this verbose comment line exists only to outweigh snip's fixed guidance banner";

/** A line-comment fixture: a thick comment block (marker inside) + trivial code. */
function lineSrc(token, code, prefix = "") {
  const lines = [`${token} ${MARKER} must never reach the model — ${FILLER}`];
  for (let i = 2; i <= COMMENT_LINES; i++) lines.push(`${token} ${FILLER} (line ${i})`);
  return `${prefix}${lines.join("\n")}\n${code}\n`;
}

/** A block-comment fixture (for languages with no line comment: CSS, HTML). */
function blockSrc(open, close, code) {
  const lines = [`${open} ${MARKER} must never reach the model — ${FILLER}`];
  for (let i = 2; i <= COMMENT_LINES; i++) lines.push(`   ${FILLER} (line ${i})`);
  return `${lines.join("\n")} ${close}\n${code}\n`;
}

export const LANGS = [
  { name: "rust", ext: "rs", src: lineSrc("//", "fn main() {}") },
  { name: "python", ext: "py", src: lineSrc("#", "x = 1") },
  { name: "javascript", ext: "js", src: lineSrc("//", "const x = 1;") },
  { name: "typescript", ext: "ts", src: lineSrc("//", "const x: number = 1;") },
  { name: "tsx", ext: "tsx", src: lineSrc("//", "const x = 1;") },
  { name: "go", ext: "go", src: lineSrc("//", "package main\n\nfunc main() {}") },
  { name: "c", ext: "c", src: lineSrc("//", "int main(void) { return 0; }") },
  { name: "cpp", ext: "cpp", src: lineSrc("//", "int main() { return 0; }") },
  { name: "java", ext: "java", src: lineSrc("//", "class A {}") },
  { name: "ruby", ext: "rb", src: lineSrc("#", "x = 1") },
  { name: "bash", ext: "sh", src: lineSrc("#", "x=1") },
  { name: "csharp", ext: "cs", src: lineSrc("//", "class A {}") },
  { name: "php", ext: "php", src: lineSrc("//", "$x = 1;", "<?php\n") },
  { name: "css", ext: "css", src: blockSrc("/*", "*/", ".a { color: red; }") },
  { name: "lua", ext: "lua", src: lineSrc("--", "local x = 1") },
  { name: "elixir", ext: "ex", src: lineSrc("#", "x = 1") },
  { name: "kotlin", ext: "kt", src: lineSrc("//", "val x = 1") },
  { name: "scala", ext: "scala", src: lineSrc("//", "val x = 1") },
  { name: "yaml", ext: "yaml", src: lineSrc("#", "key: value") },
  { name: "toml", ext: "toml", src: lineSrc("#", 'key = "value"') },
  { name: "sql", ext: "sql", src: lineSrc("--", "SELECT 1;") },
  { name: "html", ext: "html", src: blockSrc("<!--", "-->", "<div>x</div>") },
  { name: "swift", ext: "swift", src: lineSrc("//", "let x = 1") },
  { name: "dart", ext: "dart", src: lineSrc("//", "var x = 1;") },
  { name: "r", ext: "r", src: lineSrc("#", "x <- 1") },
  { name: "zig", ext: "zig", src: lineSrc("//", "const x = 1;") },
  { name: "julia", ext: "jl", src: lineSrc("#", "x = 1") },
  { name: "haskell", ext: "hs", src: lineSrc("--", "x = 1") },
  { name: "objc", ext: "m", src: lineSrc("//", "int main(void) { return 0; }") },
];

export const SECRET_MARKER = MARKER;
