const CHAT_API_URL = "https://chat-api.8b8t.me";
const CHAT_TOKEN_KEY = "b8chatbridge.session";
const SERVER_STATUS_URL = "https://api.mcstatus.io/v2/status/java/8b8t.me";

let currentUser = null;
let chatSocket = null;
let reconnectTimer = null;
let captchaConfig = null;
let captchaWidgetId = null;

document.addEventListener("DOMContentLoaded", () => {
  bindForms();
  loadServerStatus();
  bootChatPage();
});

function bindForms() {
  document.getElementById("setup-form").addEventListener("submit", handleSetupSubmit);
  document.getElementById("login-form").addEventListener("submit", handleLoginSubmit);
  document.getElementById("message-form").addEventListener("submit", handleMessageSubmit);
  document.getElementById("logout-btn").addEventListener("click", handleLogout);
}

async function loadServerStatus() {
  const count = document.getElementById("server-online-count");
  const label = document.getElementById("server-online-label");
  const dot = document.getElementById("server-online-dot");
  if (!count || !label || !dot) return;

  try {
    const res = await fetch(SERVER_STATUS_URL);
    const data = await res.json();
    const online = data.online === true;
    const players = data.players?.online ?? 0;
    const max = data.players?.max;

    count.textContent = online ? compactNumber(players) : "0";
    label.textContent = online && max ? `of ${compactNumber(max)} players online` : "players online";
    dot.classList.toggle("online", online);
    dot.classList.toggle("offline", !online);
  } catch (error) {
    count.textContent = "—";
    label.textContent = "status unavailable";
    dot.classList.add("offline");
  }
}

function compactNumber(value) {
  if (!value) return "0";
  if (value >= 1000000) return `${(value / 1000000).toFixed(1)}M`;
  if (value >= 1000) return `${(value / 1000).toFixed(1)}K`;
  return value.toString();
}

async function bootChatPage() {
  const setupToken = getSetupTokenFromUrl();
  if (setupToken) {
    showOnly("setup-card");
    await loadSetupInfo(setupToken);
    return;
  }

  const token = getToken();
  if (!token) {
    showOnly("login-card");
    return;
  }

  try {
    const data = await apiFetch("/auth/me", { token });
    currentUser = data.user || data;
    await showCaptchaOrChat();
  } catch (error) {
    clearToken();
    showOnly("login-card");
  }
}

async function loadSetupInfo(setupToken) {
  const status = document.getElementById("setup-status");
  try {
    const data = await apiFetch(`/auth/setup/${encodeURIComponent(setupToken)}`);
    document.getElementById("setup-username").value = data.username;
    document.getElementById("setup-form").dataset.token = setupToken;
    status.hidden = true;
  } catch (error) {
    showStatus(status, error.message || "This setup link is invalid or expired.", "error");
    document.getElementById("setup-form").querySelector("button").disabled = true;
  }
}

async function handleSetupSubmit(event) {
  event.preventDefault();
  const form = event.currentTarget;
  const status = document.getElementById("setup-status");
  const button = form.querySelector("button");
  button.disabled = true;

  try {
    const data = await apiFetch("/auth/setup", {
      method: "POST",
      body: {
        token: form.dataset.token,
        password: document.getElementById("setup-password").value,
      },
    });
    setToken(data.token);
    currentUser = data.user;
    history.replaceState(null, "", "/chat");
    await showCaptchaOrChat();
  } catch (error) {
    showStatus(status, error.message || "Could not set your password.", "error");
  } finally {
    button.disabled = false;
  }
}

function getSetupTokenFromUrl() {
  const paramsToken = new URLSearchParams(window.location.search).get("setup");
  if (paramsToken) {
    return paramsToken;
  }

  const path = window.location.pathname.replace(/\/+$/, "");
  const parts = path.split("/").filter(Boolean);
  const last = parts[parts.length - 1] || "";

  if (parts.length >= 2 && parts[parts.length - 2] === "chat" && last !== "chat") {
    return last;
  }

  if (last.startsWith("chat") && last.length > "chat".length) {
    return last.slice("chat".length);
  }

  return null;
}

async function handleLoginSubmit(event) {
  event.preventDefault();
  const status = document.getElementById("login-status");
  const button = event.currentTarget.querySelector("button");
  button.disabled = true;

  try {
    const data = await apiFetch("/auth/login", {
      method: "POST",
      body: {
        username: document.getElementById("login-username").value.trim(),
        password: document.getElementById("login-password").value,
      },
    });
    setToken(data.token);
    currentUser = data.user;
    await showCaptchaOrChat();
  } catch (error) {
    showStatus(status, error.message || "Invalid username or password.", "error");
  } finally {
    button.disabled = false;
  }
}

async function showChat() {
  showOnly("chat-panel");
  document.getElementById("signed-in-label").textContent = `Signed in as ${currentUser.username}`;
  await loadHistory();
  connectChatSocket();
}

async function showCaptchaOrChat() {
  const config = await loadCaptchaConfig();
  if (!config.enabled) {
    await showChat();
    return;
  }

  showOnly("captcha-card");
  document.getElementById("captcha-status").hidden = true;
  await renderCaptcha(config.site_key);
}

async function loadCaptchaConfig() {
  if (!captchaConfig) {
    captchaConfig = await apiFetch("/captcha/config");
  }
  return captchaConfig;
}

function loadCaptchaScript() {
  if (window.hcaptcha) {
    return Promise.resolve();
  }

  return new Promise((resolve, reject) => {
    const existing = document.querySelector('script[src^="https://js.hcaptcha.com/1/api.js"]');
    if (existing) {
      existing.addEventListener("load", resolve, { once: true });
      existing.addEventListener("error", reject, { once: true });
      return;
    }

    const script = document.createElement("script");
    script.src = "https://js.hcaptcha.com/1/api.js?render=explicit";
    script.async = true;
    script.defer = true;
    script.onload = resolve;
    script.onerror = reject;
    document.head.append(script);
  });
}

async function renderCaptcha(siteKey) {
  const status = document.getElementById("captcha-status");
  const container = document.getElementById("captcha-container");
  container.textContent = "";
  captchaWidgetId = null;

  try {
    await loadCaptchaScript();
    captchaWidgetId = window.hcaptcha.render(container, {
      sitekey: siteKey,
      callback: verifyCaptcha,
      "expired-callback": () => showStatus(status, "hCaptcha expired. Please solve it again.", "error"),
      "error-callback": () => showStatus(status, "hCaptcha failed to load. Please try again.", "error"),
    });
  } catch (error) {
    showStatus(status, "Could not load hCaptcha. Refresh and try again.", "error");
  }
}

async function verifyCaptcha(response) {
  const status = document.getElementById("captcha-status");
  showStatus(status, "Verifying hCaptcha...", "");
  try {
    await apiFetch("/captcha/verify", {
      method: "POST",
      token: getToken(),
      body: { response },
    });
    await showChat();
  } catch (error) {
    showStatus(status, error.message || "hCaptcha verification failed.", "error");
    if (window.hcaptcha && captchaWidgetId !== null) {
      window.hcaptcha.reset(captchaWidgetId);
    }
  }
}

async function loadHistory() {
  const historyEl = document.getElementById("chat-history");
  historyEl.textContent = "";
  try {
    const messages = await apiFetch("/chat/history?limit=100", { token: getToken() });
    messages.forEach(appendMessage);
    scrollHistoryToBottom();
  } catch (error) {
    showStatus(document.getElementById("chat-status"), "Could not load chat history.", "error");
  }
}

function connectChatSocket() {
  if (chatSocket) {
    chatSocket.close();
  }
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
  }

  const wsUrl = `${CHAT_API_URL.replace(/^https:/, "wss:").replace(/^http:/, "ws:")}/chat/ws?token=${encodeURIComponent(getToken())}`;
  chatSocket = new WebSocket(wsUrl);

  chatSocket.addEventListener("message", (event) => {
    try {
      appendMessage(JSON.parse(event.data));
      scrollHistoryToBottom();
    } catch (error) {
      // Ignore malformed messages.
    }
  });

  chatSocket.addEventListener("close", () => {
    reconnectTimer = setTimeout(() => {
      if (getToken() && !document.getElementById("chat-panel").hidden) {
        connectChatSocket();
      }
    }, 5000);
  });
}

async function handleMessageSubmit(event) {
  event.preventDefault();
  const input = document.getElementById("message-input");
  const button = event.currentTarget.querySelector("button");
  const message = input.value.trim();
  if (!message) return;

  button.disabled = true;
  try {
    const sentMessage = await apiFetch("/chat/message", {
      method: "POST",
      token: getToken(),
      body: { message },
    });
    appendMessage(sentMessage);
    scrollHistoryToBottom();
    input.value = "";
  } catch (error) {
    showStatus(document.getElementById("chat-status"), error.message || "Could not send message.", "error");
  } finally {
    button.disabled = false;
    input.focus();
  }
}

async function handleLogout() {
  try {
    await apiFetch("/auth/logout", { method: "POST", token: getToken() });
  } catch (error) {
    // Local logout still happens if the API is unavailable.
  }
  if (chatSocket) {
    chatSocket.close();
    chatSocket = null;
  }
  currentUser = null;
  clearToken();
  showOnly("login-card");
}

async function apiFetch(path, options = {}) {
  const headers = { Accept: "application/json" };
  if (options.body) {
    headers["Content-Type"] = "application/json";
  }
  if (options.token) {
    headers.Authorization = `Bearer ${options.token}`;
  }

  const response = await fetch(`${CHAT_API_URL}${path}`, {
    method: options.method || "GET",
    headers,
    body: options.body ? JSON.stringify(options.body) : undefined,
  });

  let payload = null;
  try {
    payload = await response.json();
  } catch (error) {
    throw new Error("Chat API returned an invalid response.");
  }

  if (!response.ok || !payload.success) {
    throw new Error(payload.error || "Chat API request failed.");
  }
  return payload.data;
}

function appendMessage(message) {
  const historyEl = document.getElementById("chat-history");
  if (historyEl.querySelector(`[data-message-id="${message.id}"]`)) {
    return;
  }

  const row = document.createElement("article");
  row.className = `chat-message ${message.source === "website" ? "website" : "minecraft"}`;
  row.dataset.messageId = message.id;

  const avatar = document.createElement("img");
  avatar.className = "chat-message-avatar";
  avatar.src = getPlayerHeadUrl(message.username);
  avatar.alt = `${message.username || "Player"} head`;
  avatar.loading = "lazy";
  avatar.onerror = () => {
    avatar.onerror = null;
    avatar.src = getPlayerHeadUrl("Steve");
  };

  const body = document.createElement("div");
  body.className = "chat-message-body";

  const meta = document.createElement("div");
  meta.className = "chat-message-meta";

  const name = document.createElement("span");
  name.className = "chat-message-name";
  name.textContent = message.username || "Unknown";

  const source = document.createElement("span");
  source.className = "chat-message-source";
  source.textContent = message.source || "chat";

  const time = document.createElement("time");
  time.textContent = formatMessageTime(message.created_at);

  const text = document.createElement("div");
  text.className = "chat-message-text";
  text.textContent = message.message || "";

  meta.append(name, source, time);
  body.append(meta, text);
  row.append(avatar, body);
  historyEl.append(row);
}

function getPlayerHeadUrl(username) {
  return `https://mc-heads.net/avatar/${encodeURIComponent(username || "Steve")}`;
}

function showOnly(id) {
  ["setup-card", "login-card", "captcha-card", "chat-panel"].forEach((item) => {
    document.getElementById(item).hidden = item !== id;
  });
}

function showStatus(element, message, type) {
  element.textContent = message;
  element.className = `chat-status ${type || ""}`.trim();
  if (element.id === "chat-status") {
    element.classList.add("compact");
  }
  element.hidden = false;
}

function scrollHistoryToBottom() {
  const historyEl = document.getElementById("chat-history");
  historyEl.scrollTop = historyEl.scrollHeight;
}

function formatMessageTime(timestamp) {
  if (!timestamp) return "";
  return new Date(timestamp * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function getToken() {
  return localStorage.getItem(CHAT_TOKEN_KEY);
}

function setToken(token) {
  localStorage.setItem(CHAT_TOKEN_KEY, token);
}

function clearToken() {
  localStorage.removeItem(CHAT_TOKEN_KEY);
}
