# AppContainer network-isolation proof

**Status:** Phase 3C3C2C4 native security gate; no public high-risk process admission and no network capability is introduced.

## Purpose

The October adversarial audit correctly distinguished policy intent from OS-enforced network denial. A request carrying `network=false` is not, by itself, proof that Windows prevents a child process from opening sockets.

Optic's high-risk Windows path now uses an ephemeral AppContainer whose `SECURITY_CAPABILITIES` contains the profile SID and **zero capability SIDs**. This gate tests the concrete OS behavior of that exact path with a representative interpreter that is already known to run correctly there: Node.js.

## Native test

The Windows integration test:

1. binds a host TCP listener to an ephemeral `127.0.0.1` port;
2. proves the host listener itself accepts a normal host connection;
3. launches Node through the real `ProcessManager -> isolation launcher -> AppContainer` path as `Interpreter`;
4. grants no workspace files, environment variables or network capabilities;
5. makes the Node script attempt a TCP connection to the host listener;
6. requires the script to reach the network attempt but never report `NETWORK_CONNECTED`;
7. requires a non-success exit caused by socket error or a bounded connection timeout;
8. independently requires the host listener to observe **no accepted AppContainer connection**.

The test is fail-closed: if the listener accepts the AppContainer connection, CI fails.

## Scope of the claim

A passing test proves only the property actually exercised: the current capability-free AppContainer path denies this representative Node TCP loopback connection on the supported Windows CI environment.

It does **not** by itself prove:

- every Internet/intranet protocol or address family is denied;
- UDP behavior;
- behavior on a machine with an externally configured AppContainer loopback exemption;
- network containment for the direct non-AppContainer `FixedTool` path;
- future positive network-capability behavior.

Therefore the global audit finding about `network=false` must not be marked universally resolved merely from this gate. It can, however, become a mechanically tested guarantee for the high-risk AppContainer path covered by the test.

## No authority change

This gate does not add `NetworkAccess`, `internetClient`, `internetClientServer`, `privateNetworkClientServer`, firewall rules or any other network authority. It only proves the deny-by-default behavior of the existing zero-capability AppContainer configuration.
