# LLM integration

All three clients use the same kernel jobs and CAS source validator. Profiles are editable data and support OpenAI-compatible chat/FIM, Anthropic, DeepSeek FIM and Ollama FIM. Browser transport uses fetch; desktop and CLI use native HTTP. Models propose source; the CAS parses suggestions and executes only after an explicit action.

API keys never belong in notebooks or logs. Native credentials resolve in the order configured environment variable, system vault, legacy plaintext fallback. The browser keeps credentials in memory unless the user explicitly chooses to remember them. Tests use synthetic credentials and loopback/intercepted HTTP.

The terminal shares the desktop configuration location. Inspect it with `om config path` or `om config show`; show masks keys, headers and nested string parameters. `om config edit` opens VISUAL/EDITOR as literal arguments. Restart the terminal after editing. `--config PATH` selects an isolated file; `--no-config` disables durable configuration and history.

```sh
om llm test deepseek
om --json llm test deepseek
```

A probe prints the actual provider reply and measured total/first-byte time; failures return a nonzero status. In the REPL, `? QUESTION` or `:ask QUESTION` displays parsed suggestions and waits for `y` to evaluate, `n` to cancel, or `e` to edit. Edit inserts source into the terminal buffer without evaluating it. `:explain` streams an explanation of the last retained computation. Requests disclose their destination and context and ask for confirmation.

History hints are enabled by default. Background AI hints require explicit opt-in:

```toml
[cli]
ai_hints = true
```

After first-use confirmation, a separate worker waits for 350 ms of idle input and requests the configured completion profile. Key handling reads only a nonblocking cache. Changed input cancels obsolete requests; default/omitted CLI preferences send no AI hint requests.

The protocol, privacy controls and provider options are specified in [PLAN.md §11](plan/PLAN.md#11-llm-层规格om-llm) and [protocol.md](protocol.en.md).

## Notebook settings and transport

Models, URLs, capability flags and provider parameters are editable; preset names never determine runtime routing. General settings include language/theme/dialect, constant mode, reactive execution, steps, automatic plots and evaluation timeout. Test connection uses the unsaved draft without writing configuration, notebook or vault data. A profile may explicitly disable the API-key requirement for a keyless service. Steps, Ask, chat and completion use the same profile-readiness rules.

The browser can store nonsecret settings separately from credentials; selecting Remember credentials explicitly stores the opted-in provider values in browser storage. Native configuration files use atomic writes, and new keys go to the vault. The configured environment variable has priority. Key presence is resolved by the native host; merely naming an environment variable does not establish that a key exists.

Native HTTP errors and browser CORS failures are surfaced rather than treated as successful completions. Cancellation ignores stale replies. Browser calculation interruption terminates the WASM worker, restores source and symbol definitions, and leaves other output stale; native cancellation directly signals the running computation. LLM completion accepts only a checked insertion, and translation/fix cards use the actual CAS parser. Model explanations remain model text; mathematical verification comes from the solver's retained evidence.

DeepSeek defaults use the editable `deepseek-flash` model and explicit thinking controls. Provider setup follows the [official DeepSeek first-call documentation](https://api-docs.deepseek.com/). A user-authorized live smoke test previously exercised probe, translation, completion and chat through the shared kernel; all automated and packaged acceptance tests use synthetic providers. No live key is included in examples, fixtures or notebook files.
