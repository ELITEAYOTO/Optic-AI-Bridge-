# Error Model

Stable categories should include:

InvalidRequest
Unsupported
PermissionDenied
ApprovalRequired
PathOutsideGrant
UnsafeTarget
NotFound
AlreadyExists
Conflict
StaleBase
SessionExpired
WrongSession
JobNotFound
JobFinished
Timeout
Cancelled
ResourceLimit
OutputTruncated
TransportUnavailable
RecoveryRequired
Internal

Errors should carry a safe human-readable message and structured metadata needed for recovery. Never expose secrets, raw tokens or unnecessary absolute paths.
