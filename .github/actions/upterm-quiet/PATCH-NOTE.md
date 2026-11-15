# upterm-quiet — patched owenthereal/action-upterm

Upstream: https://github.com/owenthereal/action-upterm @ 6c2d60e09009f59cb4c45c88767c78701f14d121 (v2 line)

Change: in waitForSession() the poll loop printed the session status + full ssh connect info every 5 s (src/index.ts:1140); ~70-90 lines/min flooded job logs during long waits. Patched lib/index.js prints it once (flag `announced`); state-change logs (guest joined, outages, timeout, quit) are untouched.

Applied only to the committed bundle lib/index.js; action.yml copied verbatim (node24 runtime).
