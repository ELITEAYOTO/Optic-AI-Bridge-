# FranceStudent persistent dummy auth — 2026-10-13

Source: operator-provided FranceStudent Luna results and local server logs in the
guided test. Tested code #189 head 9e96ea0185dc59aa39e943cf2d5368dc3e13ca80.
All credentials were published dummy values. Authority NONE, one remote metadata
tool, no real workspace or production credentials. No raw bearer values recorded
here. Operator confirmed server/tunnel stopped and connector disabled at end.

## Finite matched evidence

* Windows setup succeeded; all11 native tests and disposable actual process-kill/
  restart MCP smoke passed. Explicit manual fixture initialization succeeded once.
* Initial server2745fc5a4004: credential probe sequence7, marker283d0de5fb7f0623,
  principal d6b996b6507ff4aa, epoch1, oktrue. Marker matched local TOOL/status200.
* Local rotate-a ACK: active true, epoch2, persisted true. Operator stopped server
  using Ctrl+C and restarted same folder/fixture, keeping tunnel and old A config.
* New server1b3526df7a3c: sequences1POST/2DELETE/3GET denied401 at dummy_bearer,
  without DISPATCH/TOOL. Both custom headers matched; Bearer present/single but
  not currently accepted. Luna returned actual initialize Unauthorized error.
* After operator configured rotated A dummy, initialize/initialized/tools-list
  succeeded. Probe sequence10 markerd548b4ae5fb59f25 matched local TOOL/status200:
  SAME principal d6b996b6507ff4aa, epoch2, oktrue.
* Local revoke-a ACK: active false, epoch3, persisted true. Ctrl+C and restart
  same fixture, retaining A-v2 host configuration.
* New server401db94b2c4e: sequences1POST/2DELETE/3GET denied401 at dummy_bearer,
  no DISPATCH/TOOL; custom headers still matched. Luna initialize Unauthorized.
* Operator switched to initial B dummy. Discovery succeeded; probe sequence10
  marker749e4b93cc043520 matched local TOOL/status200, principal78a2b60b9cc27834,
  epoch1, oktrue. B stayed usable with a distinct principal after A revocation.
* Accepted probes and supplied discovery requests had single Bearer and both
  matching dummy headers. Accepted DELETE cleanup returned405 at stage mcp;
  this is neither cancellation nor a revocation failure.

## Conclusions and limits

Acknowledged rotation and revocation survived the operator's process stop/restart
when accessed through FranceStudent. Rotation preserved A principal and advanced
epoch; B remained independently usable after A revoke. New instance identifiers
confirm different server instances. Exact HTTP socket reuse/host reconnect policy
is not established. UI credential changes are operator testimony, not raw capture.

These manual restarts used Ctrl+C, not an abrupt kill or power cut. Actual kill is
covered separately by local automation. This is not production secret persistence,
authenticated/rollback-resistant storage, DPAPI, recovery or power-loss durability.
Production H-02/H-03 gates remain open; no draft merge or real authority enabled.
H-01 host parallel overlap, host timeout/cancellation and conversation isolation
remain unmeasured. #178 must not be conflated with H-01. No additional user action
is needed for this completed manual persistence series.
