--------------------------- MODULE ClientAssets ---------------------------
\* Capability tickets for client-half assets (/v1/plugins/{name}/asset/...).
\*
\* A ticket is issued for one plugin, addresses only paths inside that plugin
\* package, and expires. The model checks the three boundaries that matter:
\* scope (inside paths), provenance (issued caps), and time (TTL).
\*
\* Requests are a bounded history so TLC can enumerate them.
EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS Caps, InsidePaths, OutsidePaths, TTL, MaxTick, MaxRequests

VARIABLES now, issued, history

Paths == InsidePaths \cup OutsidePaths

vars == <<now, issued, history>>

Init ==
    /\ now = 0
    /\ issued = {}
    /\ history = <<>>

IssuedCaps == {r.cap : r \in issued}

Expiry(c) ==
    IF c \in IssuedCaps
    THEN (CHOOSE r \in issued: r.cap = c).expires
    ELSE 0

Tick ==
    /\ now < MaxTick
    /\ now' = now + 1
    /\ UNCHANGED <<issued, history>>

Issue(c) ==
    /\ c \in Caps
    /\ c \notin IssuedCaps
    /\ issued' = issued \cup {[cap |-> c, at |-> now, expires |-> now + TTL]}
    /\ UNCHANGED <<now, history>>

Attempt(c, path) ==
    \* Any request may arrive; `allowed` records the server's decision.
    /\ Len(history) < MaxRequests
    /\ c \in Caps
    /\ path \in Paths
    /\ LET valid == c \in IssuedCaps /\ path \in InsidePaths /\ now <= Expiry(c)
       IN history' = Append(history,
            [cap |-> c, path |-> path, at |-> now, allowed |-> valid])
    /\ UNCHANGED <<now, issued>>

Next ==
    \/ Tick
    \/ (\E c \in Caps: Issue(c))
    \/ (\E c \in Caps, path \in Paths: Attempt(c, path))

Spec == Init /\ [][Next]_vars

Requests == {history[i] : i \in 1..Len(history)}
Served == {a \in Requests : a.allowed}

TypeInvariant ==
    /\ now \in 0..MaxTick
    /\ \A r \in issued: r.cap \in Caps /\ r.at \in 0..MaxTick /\ r.expires \in 0..MaxTick + TTL
    /\ Len(history) <= MaxRequests
    /\ \A a \in Requests: a.cap \in Caps /\ a.path \in Paths /\ a.at \in 0..MaxTick

\* A ticket is issued once, for an exact TTL.
OneTicketPerCapability == Cardinality(IssuedCaps) = Cardinality(issued)

IssuedExactness ==
    \A r \in issued: r.expires = r.at + TTL

\* Served bytes always come from an issued capability, inside the package,
\* and before the capability expired.
ServedIssued ==
    \A a \in Served: a.cap \in IssuedCaps

ServedConfined ==
    \A a \in Served: a.path \in InsidePaths

ServedWithinTTL ==
    \A a \in Served: a.at <= Expiry(a.cap)

\* Negative boundaries: outside paths, unknown capabilities and late requests
\* are never allowed.
OutsideNeverAllowed ==
    \A a \in Requests: a.path \in OutsidePaths => ~a.allowed

UnissuedNeverAllowed ==
    \A a \in Requests: a.cap \notin IssuedCaps => ~a.allowed

LateNeverAllowed ==
    \A a \in Requests: a.at > Expiry(a.cap) => ~a.allowed

Safety ==
    /\ TypeInvariant
    /\ OneTicketPerCapability
    /\ IssuedExactness
    /\ ServedIssued
    /\ ServedConfined
    /\ ServedWithinTTL
    /\ OutsideNeverAllowed
    /\ UnissuedNeverAllowed
    /\ LateNeverAllowed

===========================================================================
