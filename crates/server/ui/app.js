const state = {
  token: localStorage.getItem("dshr-token") || "",
  session: null,
  source: null,
  approvalId: null,
};

const $ = (id) => document.getElementById(id);

function headers() {
  return { "Authorization": `Bearer ${state.token}`, "Content-Type": "application/json" };
}

async function api(path, options = {}) {
  const response = await fetch(path, { ...options, headers: headers() });
  if (!response.ok) throw new Error(`${response.status} ${await response.text()}`);
  return response.json();
}

function addMessage(role, text) {
  const node = document.createElement("div");
  node.className = `message ${role}`;
  node.textContent = text;
  $("messages").appendChild(node);
  node.scrollIntoView({ block: "end" });
  return node;
}

function addTool(text) {
  const node = document.createElement("div");
  node.className = "tool";
  node.textContent = text;
  $("messages").appendChild(node);
}

async function loadSessions() {
  const data = await api("/v1/sessions");
  const list = $("sessions");
  list.innerHTML = "";
  for (const session of data.sessions) {
    const button = document.createElement("button");
    button.className = "session";
    button.textContent = `${session.id.slice(0, 12)} · ${session.turns} turn(s)`;
    button.onclick = () => selectSession(session.id);
    list.appendChild(button);
  }
}

async function selectSession(id) {
  state.session = id;
  const data = await api(`/v1/sessions/${id}`);
  $("messages").innerHTML = "";
  for (const message of data.messages) {
    const text = message.content.filter((block) => block.type === "text").map((block) => block.text).join("");
    if (text) addMessage(message.role, text);
  }
  if (state.source) state.source.close();
  state.source = new EventSource(`/v1/sessions/${id}/events?token=${encodeURIComponent(state.token)}`);
  state.source.onmessage = (event) => handleEvent(JSON.parse(event.data));
  $("status").textContent = id;
}

function handleEvent(event) {
  switch (event.type) {
    case "text": {
      let last = $("messages").lastElementChild;
      if (!last || !last.classList.contains("assistant") || last.dataset.streaming !== "1") {
        last = addMessage("assistant", "");
        last.dataset.streaming = "1";
      }
      last.textContent += event.text;
      break;
    }
    case "tool_start": addTool(`→ ${event.name} ${event.summary}`); break;
    case "tool_end": addTool(`← ${event.name} ${event.ok ? "ok" : "error"}`); break;
    case "approval":
      state.approvalId = event.id;
      $("approval-text").textContent = `${event.tool}: ${event.summary}\n${event.reason}`;
      $("approval").showModal();
      break;
    case "done":
      $("status").textContent = `${event.usage.input_tokens} in / ${event.usage.output_tokens} out`;
      $("messages").lastElementChild?.removeAttribute("data-streaming");
      break;
    case "error":
      addMessage("error", event.message);
      break;
    case "cancelled":
      addMessage("error", "cancelled");
      break;
  }
}

async function createSession() {
  const data = await api("/v1/sessions", { method: "POST", body: "{}" });
  await loadSessions();
  await selectSession(data.session.id);
}

async function sendPrompt() {
  if (!state.session) return;
  const prompt = $("prompt").value.trim();
  if (!prompt) return;
  addMessage("user", prompt);
  $("prompt").value = "";
  await api(`/v1/sessions/${state.session}/turns`, {
    method: "POST",
    body: JSON.stringify({ prompt }),
  });
}

async function resolveApproval(approved) {
  if (!state.approvalId) return;
  await api(`/v1/sessions/${state.session}/approvals/${state.approvalId}`, {
    method: "POST",
    body: JSON.stringify({ approved }),
  });
  state.approvalId = null;
  $("approval").close();
}

$("connect").onclick = async () => {
  state.token = $("token").value.trim();
  localStorage.setItem("dshr-token", state.token);
  await loadSessions();
  $("status").textContent = "connected";
};
$("new-session").onclick = () => createSession().catch((error) => addMessage("error", error.message));
$("prompt-form").onsubmit = (event) => {
  event.preventDefault();
  sendPrompt().catch((error) => addMessage("error", error.message));
};
$("cancel").onclick = () => state.session && api(`/v1/sessions/${state.session}/cancel`, { method: "POST", body: "{}" });
$("approve").onclick = () => resolveApproval(true);
$("deny").onclick = () => resolveApproval(false);

$("token").value = state.token;
if (state.token) $("connect").click();
