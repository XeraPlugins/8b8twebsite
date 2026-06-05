package ca.xera.b8chatbridge;

import com.google.gson.Gson;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;

import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.WebSocket;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionStage;
import java.util.function.Consumer;

public final class ChatBridgeApiClient {
    private final HttpClient httpClient;
    private final Gson gson;
    private final String apiBaseUrl;
    private final String pluginSecret;

    private volatile WebSocket webSocket;

    public ChatBridgeApiClient(String apiBaseUrl, String pluginSecret) {
        this.apiBaseUrl = trimTrailingSlash(apiBaseUrl);
        this.pluginSecret = pluginSecret;
        this.gson = new Gson();
        this.httpClient = HttpClient.newBuilder()
                .connectTimeout(Duration.ofSeconds(10))
                .build();
    }

    public CompletableFuture<SetupLinkResponse> requestSetupLink(UUID uuid, String username, boolean reset) {
        JsonObject body = new JsonObject();
        body.addProperty("uuid", uuid.toString());
        body.addProperty("username", username);
        body.addProperty("reset", reset);

        HttpRequest request = baseRequest("/plugin/setup-link")
                .POST(HttpRequest.BodyPublishers.ofString(gson.toJson(body)))
                .build();

        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(response -> parseApiResponse(response, SetupLinkResponse.class));
    }

    public CompletableFuture<Void> sendMinecraftMessage(UUID uuid, String username, String message) {
        JsonObject body = new JsonObject();
        body.addProperty("uuid", uuid.toString());
        body.addProperty("username", username);
        body.addProperty("message", message);

        HttpRequest request = baseRequest("/plugin/minecraft-message")
                .POST(HttpRequest.BodyPublishers.ofString(gson.toJson(body)))
                .build();

        return httpClient.sendAsync(request, HttpResponse.BodyHandlers.ofString())
                .thenApply(response -> {
                    if (response.statusCode() < 200 || response.statusCode() >= 300) {
                        throw new ChatBridgeApiException("API returned HTTP " + response.statusCode());
                    }
                    return null;
                });
    }

    public CompletableFuture<Void> connectPluginWebSocket(Consumer<ChatMessage> onMessage, Runnable onClose) {
        String wsUrl = toWebSocketUrl(apiBaseUrl) + "/plugin/ws?secret=" + urlEncode(pluginSecret);
        return httpClient.newWebSocketBuilder()
                .connectTimeout(Duration.ofSeconds(10))
                .buildAsync(URI.create(wsUrl), new WebSocket.Listener() {
                    private final StringBuilder partial = new StringBuilder();

                    @Override
                    public void onOpen(WebSocket webSocket) {
                        ChatBridgeApiClient.this.webSocket = webSocket;
                        webSocket.request(1);
                    }

                    @Override
                    public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
                        partial.append(data);
                        if (last) {
                            String payload = partial.toString();
                            partial.setLength(0);
                            try {
                                ChatMessage chatMessage = gson.fromJson(payload, ChatMessage.class);
                                if (chatMessage != null) {
                                    onMessage.accept(chatMessage);
                                }
                            } catch (RuntimeException ignored) {
                            }
                        }
                        webSocket.request(1);
                        return CompletableFuture.completedFuture(null);
                    }

                    @Override
                    public CompletionStage<?> onClose(WebSocket webSocket, int statusCode, String reason) {
                        ChatBridgeApiClient.this.webSocket = null;
                        onClose.run();
                        return CompletableFuture.completedFuture(null);
                    }

                    @Override
                    public void onError(WebSocket webSocket, Throwable error) {
                        ChatBridgeApiClient.this.webSocket = null;
                        onClose.run();
                    }
                })
                .thenApply(ws -> null);
    }

    public void closeWebSocket() {
        WebSocket socket = webSocket;
        if (socket != null) {
            socket.sendClose(WebSocket.NORMAL_CLOSURE, "Plugin disabled");
            webSocket = null;
        }
    }

    private HttpRequest.Builder baseRequest(String path) {
        return HttpRequest.newBuilder(URI.create(apiBaseUrl + path))
                .timeout(Duration.ofSeconds(10))
                .header("Content-Type", "application/json")
                .header("Accept", "application/json")
                .header("x-plugin-secret", pluginSecret);
    }

    private <T> T parseApiResponse(HttpResponse<String> response, Class<T> dataClass) {
        if (response.statusCode() < 200 || response.statusCode() >= 300) {
            throw new ChatBridgeApiException("API returned HTTP " + response.statusCode());
        }

        JsonObject root = JsonParser.parseString(response.body()).getAsJsonObject();
        boolean success = root.has("success") && root.get("success").getAsBoolean();
        if (!success) {
            JsonElement error = root.get("error");
            String message = error == null || error.isJsonNull() ? "API request failed" : error.getAsString();
            throw new ChatBridgeApiException(message);
        }

        JsonElement data = root.get("data");
        if (data == null || data.isJsonNull()) {
            throw new ChatBridgeApiException("API response did not include data");
        }
        return gson.fromJson(data, dataClass);
    }

    private static String trimTrailingSlash(String value) {
        while (value.endsWith("/")) {
            value = value.substring(0, value.length() - 1);
        }
        return value;
    }

    private static String toWebSocketUrl(String value) {
        if (value.startsWith("https://")) {
            return "wss://" + value.substring("https://".length());
        }
        if (value.startsWith("http://")) {
            return "ws://" + value.substring("http://".length());
        }
        return value;
    }

    private static String urlEncode(String value) {
        return URLEncoder.encode(value, StandardCharsets.UTF_8);
    }

    public static final class ChatBridgeApiException extends RuntimeException {
        public ChatBridgeApiException(String message) {
            super(message);
        }
    }
}
