--------------------------- MODULE ApprovalGate ---------------------------
\* Tool-call approval gate of the agent loop (dshr_agent approvals):
\* a tool executes only after its approval when the policy is "ask", denial is
\* final, and terminal turns never execute again.
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS Calls, Policy

VARIABLES mode, proposed, pending, approved, denied, executed

vars == <<mode, proposed, pending, approved, denied, executed>>

Init ==
    /\ mode = "idle"
    /\ proposed = {}
    /\ pending = {}
    /\ approved = {}
    /\ denied = {}
    /\ executed = {}

Begin ==
    /\ mode = "idle"
    /\ mode' = "running"
    /\ UNCHANGED <<proposed, pending, approved, denied, executed>>

Propose(c) ==
    /\ mode = "running"
    /\ c \in Calls
    /\ c \notin proposed
    /\ proposed' = proposed \cup {c}
    /\ pending' = IF Policy = "ask" THEN pending \cup {c} ELSE pending
    /\ UNCHANGED <<mode, approved, denied, executed>>

Approve(c) ==
    /\ c \in pending
    /\ pending' = pending \ {c}
    /\ approved' = approved \cup {c}
    /\ UNCHANGED <<mode, proposed, denied, executed>>

Deny(c) ==
    /\ c \in pending
    /\ pending' = pending \ {c}
    /\ denied' = denied \cup {c}
    /\ UNCHANGED <<mode, proposed, approved, executed>>

Execute(c) ==
    /\ mode = "running"
    /\ c \in proposed
    /\ c \notin executed
    /\ c \notin denied
    /\ (Policy = "never" \/ c \in approved)
    /\ executed' = executed \cup {c}
    /\ pending' = pending \ {c}
    /\ UNCHANGED <<mode, proposed, approved, denied>>

Cancel ==
    \* Cancelling a turn rejects whatever was still waiting for a decision.
    /\ mode = "running"
    /\ mode' = "cancelled"
    /\ pending' = {}
    /\ UNCHANGED <<proposed, approved, denied, executed>>

Complete ==
    /\ mode = "running"
    /\ pending = {}
    /\ mode' = "done"
    /\ UNCHANGED <<proposed, pending, approved, denied, executed>>

Next ==
    \/ Begin \/ Cancel \/ Complete
    \/ \E c \in Calls: Propose(c) \/ Approve(c) \/ Deny(c) \/ Execute(c)

Spec == Init /\ [][Next]_vars

TypeInvariant ==
    /\ mode \in {"idle", "running", "done", "cancelled"}
    /\ proposed \subseteq Calls
    /\ pending \subseteq proposed
    /\ approved \subseteq proposed
    /\ denied \subseteq proposed
    /\ executed \subseteq proposed

\* The core guarantee: with the "ask" policy nothing runs unapproved.
NoUnapprovedExecution ==
    (Policy = "ask") => (executed \subseteq approved)

\* Denial is final and approvals/denials are disjoint.
ExecutedNotDenied == executed \cap denied = {}
ApprovalDecisionDisjoint == approved \cap denied = {}
PendingUndecided == pending \cap (approved \cup denied) = {}

\* Terminal turns do not execute: no action adds to `executed` unless the turn
\* is running, which the next-state relation enforces; this invariant checks
\* that a finished turn has no pending approvals left.
FinishedTurnClean ==
    (mode \in {"done", "cancelled"}) => pending = {}

Safety ==
    /\ TypeInvariant
    /\ NoUnapprovedExecution
    /\ ExecutedNotDenied
    /\ ApprovalDecisionDisjoint
    /\ PendingUndecided
    /\ FinishedTurnClean

\* Decision progress: every pending call is eventually approved or denied when
\* the policy asks for a decision and the user interface stays responsive.
PendingEventuallyDecided ==
    \A c \in Calls:
        (c \in pending) ~>
            (c \in approved \/ c \in denied \/ mode \in {"done", "cancelled"})

Fairness == \A c \in Calls: WF_vars(Approve(c)) /\ WF_vars(Deny(c))

LiveSpec == Spec /\ Fairness

===========================================================================
