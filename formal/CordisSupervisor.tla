------------------------- MODULE CordisSupervisor -------------------------
\* Supervisor lifecycle for Cordis/npm plugins (dshr_cordis::CordisManager)
\* and the hot-swappable tool registry (dshr_tools::ToolRegistry).
\*
\* Abstract per plugin: its whole tool set is one registry entry, so a reload
\* is an atomic unregister/register of that entry. `registeredGen` tags the
\* generation whose tools are in the registry; in-flight calls keep the old
\* sidecar alive, so they may outlive a reload and span generations.
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS Plugins, MaxGen, AllowFailure, AllowReload

VARIABLES
    desired,        \* plugin -> "enabled" | "disabled"
    phase,          \* plugin -> "stopped" | "starting" | "running" | "failed"
    gen,            \* plugin -> generation of the running sidecar
    registered,     \* plugin -> its tools are in the registry
    registeredGen,  \* plugin -> generation of the registered tools
    inflight        \* set of [plugin |-> p, g |-> generation]

vars == <<desired, phase, gen, registered, registeredGen, inflight>>

Init ==
    /\ desired       = [p \in Plugins |-> "enabled"]
    /\ phase         = [p \in Plugins |-> "stopped"]
    /\ gen           = [p \in Plugins |-> 0]
    /\ registered    = [p \in Plugins |-> FALSE]
    /\ registeredGen = [p \in Plugins |-> 0]
    /\ inflight      = {}

Start(p) ==
    \* The implementation stops first: a stale generation's tools are dropped
    \* before the new sidecar starts connecting.
    /\ desired[p] = "enabled"
    /\ phase[p] \in {"stopped", "failed"}
    /\ phase' = [phase EXCEPT ![p] = "starting"]
    /\ registered' = [registered EXCEPT ![p] = FALSE]
    /\ UNCHANGED <<desired, gen, registeredGen, inflight>>

Connect(p) ==
    /\ phase[p] = "starting"
    /\ gen[p] < MaxGen
    /\ gen' = [gen EXCEPT ![p] = gen[p] + 1]
    /\ phase' = [phase EXCEPT ![p] = "running"]
    /\ UNCHANGED <<desired, registered, registeredGen, inflight>>

FailStart(p) ==
    /\ AllowFailure
    /\ phase[p] = "starting"
    /\ phase' = [phase EXCEPT ![p] = "failed"]
    /\ UNCHANGED <<desired, gen, registered, registeredGen, inflight>>

Register(p) ==
    /\ phase[p] = "running"
    /\ ~registered[p]
    /\ registered' = [registered EXCEPT ![p] = TRUE]
    /\ registeredGen' = [registeredGen EXCEPT ![p] = gen[p]]
    /\ UNCHANGED <<desired, phase, gen, inflight>>

Stop(p) ==
    \* Stopping is a configuration action (disable) or part of a reload.
    /\ desired[p] = "disabled"
    /\ phase[p] \in {"running", "failed", "starting"}
    /\ phase' = [phase EXCEPT ![p] = "stopped"]
    /\ registered' = [registered EXCEPT ![p] = FALSE]
    /\ UNCHANGED <<desired, gen, registeredGen, inflight>>

Reload(p) ==
    \* Atomic start of a reload: drop the registered tools, then reconnect.
    /\ AllowReload
    /\ desired[p] = "enabled"
    /\ phase[p] \in {"running", "failed"}
    /\ phase' = [phase EXCEPT ![p] = "starting"]
    /\ registered' = [registered EXCEPT ![p] = FALSE]
    /\ UNCHANGED <<desired, gen, registeredGen, inflight>>

Crash(p) ==
    \* The sidecar dies without the supervisor noticing in the same step: its
    \* tools stay registered and calls fail until the next reload/stop.
    /\ phase[p] = "running"
    /\ phase' = [phase EXCEPT ![p] = "failed"]
    /\ UNCHANGED <<desired, gen, registered, registeredGen, inflight>>

Call(p) ==
    /\ phase[p] = "running"
    /\ registered[p]
    /\ inflight' = inflight \cup {[plugin |-> p, g |-> registeredGen[p]]}
    /\ UNCHANGED <<desired, phase, gen, registered, registeredGen, inflight>>

FinishCall(call) ==
    /\ call \in inflight
    /\ inflight' = inflight \ {call}
    /\ UNCHANGED <<desired, phase, gen, registered, registeredGen, inflight>>

Disable(p) ==
    /\ desired[p] = "enabled"
    /\ desired' = [desired EXCEPT ![p] = "disabled"]
    /\ UNCHANGED <<phase, gen, registered, registeredGen, inflight>>

Enable(p) ==
    /\ desired[p] = "disabled"
    /\ desired' = [desired EXCEPT ![p] = "enabled"]
    /\ UNCHANGED <<phase, gen, registered, registeredGen, inflight>>

Next ==
    \/ \E p \in Plugins:
         \/ Start(p) \/ Connect(p) \/ FailStart(p) \/ Register(p)
         \/ Stop(p) \/ Reload(p) \/ Crash(p) \/ Call(p)
         \/ Disable(p) \/ Enable(p)
    \/ \E c \in inflight: FinishCall(c)

Spec == Init /\ [][Next]_vars

Fairness ==
    \A p \in Plugins:
        WF_vars(Start(p)) /\ WF_vars(Connect(p)) /\ WF_vars(Register(p))
        /\ WF_vars(Stop(p))

LiveSpec == Spec /\ Fairness

TypeInvariant ==
    /\ \A p \in Plugins: phase[p] \in {"stopped", "starting", "running", "failed"}
    /\ \A p \in Plugins: desired[p] \in {"enabled", "disabled"}
    /\ \A p \in Plugins: gen[p] \in 0..MaxGen
    /\ \A p \in Plugins: registeredGen[p] \in 0..MaxGen
    /\ \A c \in inflight: c.plugin \in Plugins /\ c.g \in 0..MaxGen

\* Tools are only served for a plugin that was actually started.
RegisteredImpliesStarted ==
    \A p \in Plugins: registered[p] => phase[p] \in {"running", "failed"}

\* A stopped plugin never leaves its tools behind (stop removes exactly them).
StoppedHasNoTools ==
    \A p \in Plugins: phase[p] = "stopped" => ~registered[p]

\* While a sidecar runs, its registered generation is its current generation.
RunningGenerationMatches ==
    \A p \in Plugins:
        (phase[p] = "running" /\ registered[p]) => registeredGen[p] = gen[p]

\* A generation tag never points into the future.
GenerationSound ==
    \A p \in Plugins: registeredGen[p] <= gen[p]

\* In-flight calls were started on a generation that existed; they may span
\* reloads (the old process finishes them) but never reference a future one.
InflightSound ==
    \A c \in inflight:
        c.g >= 1 /\ c.g <= gen[c.plugin] /\ c.g <= registeredGen[c.plugin]

Safety ==
    /\ TypeInvariant
    /\ RegisteredImpliesStarted
    /\ StoppedHasNoTools
    /\ RunningGenerationMatches
    /\ GenerationSound
    /\ InflightSound

\* Under fairness, configuration converges without restarting the process.
EnabledBecomesRunning ==
    \* The generation bound is a modelling budget: once exhausted the model
    \* stops connecting, which real u64 generations never do.
    \A p \in Plugins:
        (<>[](desired[p] = "enabled" /\ gen[p] < MaxGen))
            ~> (<>[](phase[p] = "running"))

DisabledBecomesStopped ==
    \A p \in Plugins:
        (<>[](desired[p] = "disabled")) ~> (<>[](phase[p] = "stopped"))

Liveness == EnabledBecomesRunning /\ DisabledBecomesStopped

=======================================================================
