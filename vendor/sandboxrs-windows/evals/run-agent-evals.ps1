param(
    [string]$OpenAiApiKey = $env:OPENAI_API_KEY,
    [int]$RunsPerScenario = 5
)

# Scaffold for the nightly/manual agentic eval. Wire this to your LLM API
# client; the deterministic suite never depends on it.
Write-Host "Agent eval scaffold: $RunsPerScenario runs per scenario"
Write-Host "Scenarios: see agent-scenarios.md"
Write-Host "API key configured: $([bool]$OpenAiApiKey)"
