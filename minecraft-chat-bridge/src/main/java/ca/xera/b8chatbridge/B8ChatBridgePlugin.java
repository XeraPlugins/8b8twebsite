package ca.xera.b8chatbridge;

import io.papermc.paper.event.player.AsyncChatEvent;
import net.kyori.adventure.text.Component;
import net.kyori.adventure.text.event.ClickEvent;
import net.kyori.adventure.text.event.HoverEvent;
import net.kyori.adventure.text.format.NamedTextColor;
import net.kyori.adventure.text.serializer.plain.PlainTextComponentSerializer;
import org.bukkit.Bukkit;
import org.bukkit.command.Command;
import org.bukkit.command.CommandExecutor;
import org.bukkit.command.CommandSender;
import org.bukkit.command.TabCompleter;
import org.bukkit.entity.Player;
import org.bukkit.event.EventHandler;
import org.bukkit.event.Listener;
import org.bukkit.plugin.java.JavaPlugin;

import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.concurrent.TimeUnit;

public final class B8ChatBridgePlugin extends JavaPlugin implements Listener, CommandExecutor, TabCompleter {
    private ChatBridgeApiClient apiClient;
    private String permissionNode;
    private String webMessageFormat;
    private boolean forwardMinecraftChat;
    private long reconnectDelaySeconds;
    private volatile boolean shuttingDown;

    @Override
    public void onEnable() {
        saveDefaultConfig();

        String apiBaseUrl = getConfig().getString("api-base-url", "https://chat-api.8b8t.me");
        String pluginSecret = getConfig().getString("plugin-secret", "");
        permissionNode = getConfig().getString("permission-node", "8b8t.chatbridge.access");
        webMessageFormat = getConfig().getString("web-message-format", "[Web] %username%: %message%");
        forwardMinecraftChat = getConfig().getBoolean("forward-minecraft-chat", true);
        reconnectDelaySeconds = Math.max(1, getConfig().getLong("reconnect-delay-seconds", 10));

        if (pluginSecret.isBlank() || pluginSecret.equals("change-this-secret")) {
            getLogger().warning("plugin-secret is not configured. Set it in plugins/b8chatbridge/config.yml.");
        }

        apiClient = new ChatBridgeApiClient(apiBaseUrl, pluginSecret);
        getServer().getPluginManager().registerEvents(this, this);

        if (getCommand("chatbridge") != null) {
            getCommand("chatbridge").setExecutor(this);
            getCommand("chatbridge").setTabCompleter(this);
        }

        connectWebSocket();
    }

    @Override
    public void onDisable() {
        shuttingDown = true;
        if (apiClient != null) {
            apiClient.closeWebSocket();
        }
    }

    @EventHandler
    public void onAsyncChat(AsyncChatEvent event) {
        if (!forwardMinecraftChat || apiClient == null) {
            return;
        }

        Player player = event.getPlayer();
        String message = PlainTextComponentSerializer.plainText().serialize(event.message());
        apiClient.sendMinecraftMessage(player.getUniqueId(), player.getName(), message)
                .exceptionally(error -> {
                    getLogger().fine("Failed to forward Minecraft chat: " + error.getMessage());
                    return null;
                });
    }

    @Override
    public boolean onCommand(CommandSender sender, Command command, String label, String[] args) {
        if (!(sender instanceof Player player)) {
            sender.sendMessage(Component.text("Only players can use /chatbridge.", NamedTextColor.RED));
            return true;
        }

        if (!player.hasPermission(permissionNode)) {
            player.sendMessage(Component.text("You do not have permission to create website chat access.", NamedTextColor.RED));
            return true;
        }

        if (args.length == 0) {
            sendSetupLink(player, false);
            return true;
        }

        String subcommand = args[0].toLowerCase(Locale.ROOT);
        switch (subcommand) {
            case "reset" -> sendSetupLink(player, true);
            case "status" -> player.sendMessage(Component.text("If you can use this command, you can create or reset website chat access.", NamedTextColor.GRAY));
            default -> sendHelp(player);
        }
        return true;
    }

    @Override
    public List<String> onTabComplete(CommandSender sender, Command command, String alias, String[] args) {
        if (args.length == 1) {
            return List.of("reset", "status");
        }
        return Collections.emptyList();
    }

    private void sendSetupLink(Player player, boolean reset) {
        player.sendMessage(Component.text(reset ? "Creating your password reset link..." : "Creating your chat bridge setup link...", NamedTextColor.GRAY));

        apiClient.requestSetupLink(player.getUniqueId(), player.getName(), reset)
                .thenAccept(response -> sendPlayerMessage(player, setupLinkMessage(response.setup_url, reset)))
                .exceptionally(error -> {
                    String message = reset
                            ? "Could not create password reset link. Try again later."
                            : "Could not create chat bridge link. If your account already exists, use /chatbridge reset.";
                    sendPlayerMessage(player, Component.text(message, NamedTextColor.RED));
                    getLogger().warning("Failed to create setup link for " + player.getName() + ": " + error.getMessage());
                    return null;
                });
    }

    private Component setupLinkMessage(String url, boolean reset) {
        String action = reset ? "reset your website chat password" : "finish creating your website chat account";
        return Component.text("Click here to " + action + ". This link expires soon.", NamedTextColor.AQUA)
                .clickEvent(ClickEvent.openUrl(url))
                .hoverEvent(HoverEvent.showText(Component.text(url, NamedTextColor.YELLOW)));
    }

    private void sendHelp(Player player) {
        player.sendMessage(Component.text("/chatbridge", NamedTextColor.AQUA)
                .append(Component.text(" - create your website chat account", NamedTextColor.GRAY)));
        player.sendMessage(Component.text("/chatbridge reset", NamedTextColor.AQUA)
                .append(Component.text(" - reset your website chat password", NamedTextColor.GRAY)));
    }

    private void sendPlayerMessage(Player player, Component message) {
        player.getScheduler().run(this, task -> {
            if (player.isOnline()) {
                player.sendMessage(message);
            }
        }, null);
    }

    private void connectWebSocket() {
        if (apiClient == null || shuttingDown) {
            return;
        }

        apiClient.connectPluginWebSocket(this::broadcastWebsiteMessage, this::scheduleReconnect)
                .exceptionally(error -> {
                    getLogger().warning("Chat bridge WebSocket failed: " + error.getMessage());
                    scheduleReconnect();
                    return null;
                });
    }

    private void scheduleReconnect() {
        if (shuttingDown) {
            return;
        }
        Bukkit.getAsyncScheduler().runDelayed(this, task -> connectWebSocket(), reconnectDelaySeconds, TimeUnit.SECONDS);
    }

    private void broadcastWebsiteMessage(ChatMessage chatMessage) {
        String line = webMessageFormat
                .replace("%username%", chatMessage.username == null ? "Website" : chatMessage.username)
                .replace("%message%", chatMessage.message == null ? "" : chatMessage.message);

        Component component = Component.text(line, NamedTextColor.LIGHT_PURPLE);
        Bukkit.getGlobalRegionScheduler().run(this, task -> Bukkit.broadcast(component));
    }
}
