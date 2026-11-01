import Vibecheck.Crypto
import Lean.Data.Json

/-!
Asking a language model, as the Python experiment does: one prompt through
the Claude Code CLI (`claude -p`), with no tools, no settings and no session.

Every answer is cached on disk under the hash of the model and the prompt,
so a rerun of the same games asks nothing new and plays out the same. The
prompt is saved beside each answer.
-/

namespace Vibecheck

structure LLM where
  model : String := "claude-opus-5-5"
  cli : String := "claude"
  cacheDir : System.FilePath := "llm-cache"

namespace LLM

def key (llm : LLM) (prompt : String) : String :=
  (Crypto.toHex (Crypto.sha512Str s!"{llm.model}\n{prompt}")).take 24

/-- The cached answer to `prompt`, or a fresh one. -/
def ask (llm : LLM) (prompt : String) : IO String := do
  let path := llm.cacheDir / s!"ask-{llm.key prompt}.txt"
  if ← path.pathExists then return ← IO.FS.readFile path
  let cwd := llm.cacheDir / ".cwd"
  IO.FS.createDirAll cwd
  let mut last := ""
  for _ in [0:3] do
    let out ← IO.Process.output {
      cmd := llm.cli
      args := #["-p", "--output-format", "text", "--model", llm.model, "--tools", "",
                "--strict-mcp-config", "--setting-sources", "", "--no-session-persistence", "--", prompt]
      cwd := cwd }
    if out.exitCode == 0 && !out.stdout.trim.isEmpty then
      let answer := out.stdout.trim
      IO.FS.writeFile (llm.cacheDir / s!"ask-{llm.key prompt}.prompt.txt") prompt
      let tmp := llm.cacheDir / s!"ask-{llm.key prompt}.tmp"
      IO.FS.writeFile tmp answer
      IO.FS.rename tmp path
      return answer
    last := out.stderr.trim
  throw (IO.userError s!"{llm.cli} failed: {last.take 500}")

end LLM

/-- The first JSON object in a reply. -/
def jsonObject? (text : String) : Option Lean.Json := do
  let start ← text.posOf '{' |> fun p => if p == text.endPos then none else some p
  let stop := text.revPosOf '}'
  let stop ← stop
  (Lean.Json.parse (text.extract start (text.next stop))).toOption

def jsonStr (j : Lean.Json) (field : String) : String :=
  (j.getObjValAs? String field).toOption.getD ""

def jsonStrs (j : Lean.Json) (field : String) : List String :=
  match j.getObjVal? field with
  | .ok (.arr xs) => xs.toList.filterMap fun x => x.getStr?.toOption
  | _ => []

end Vibecheck
