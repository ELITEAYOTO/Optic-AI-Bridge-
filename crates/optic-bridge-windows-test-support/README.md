# Windows research test support

Development fixture for the instrumented runtime unit-test host only. Not a
production process API; publish=false, Windows-only dev-dependency, no arbitrary
executable argument. The app normal dependency tree must not include it.

The fixture supplies raw Win32 helpers for a fixed synthetic self-test child
under atomic Job membership and selected stdio. The runtime retains its unsafe
code prohibition; no native functions are copied into production runtime code.
No process can run these helpers safely without the independently armed outer
Job protocol in scripts/research/issue317_authority_native_owner.py.

Native/controller Drop is an error-path best-effort backstop under that external
deadline, not proof of a bounded root destructor. Explicit keeper shutdown and
joined controller/EOF/Job proof are required by the research host's happy path.
